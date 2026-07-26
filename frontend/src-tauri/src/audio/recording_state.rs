use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::collections::VecDeque;
use tokio::sync::mpsc;
use anyhow::Result;

use super::devices::AudioDevice;
use super::buffer_pool::AudioBufferPool;

/// Global audio level data shared between audio pipeline and UI monitor.
/// Updated in real-time from the CPAL audio callback (non-blocking).
#[derive(Debug, Default, Clone)]
pub struct AudioLevelStore {
    pub mic_rms: f32,
    pub mic_peak: f32,
    pub mic_active: bool,
    pub system_rms: f32,
    pub system_peak: f32,
    pub system_active: bool,
    pub last_update_ms: u64,
}

lazy_static::lazy_static! {
    static ref AUDIO_LEVEL_STORE: Mutex<AudioLevelStore> = Mutex::new(AudioLevelStore::default());
    static ref SPECTRUM_RING: Mutex<VecDeque<f32>> = Mutex::new(VecDeque::with_capacity(SPECTRUM_RING_SIZE));
    static ref SPECTRUM_SAMPLE_RATE: Mutex<u32> = Mutex::new(48000);
}

/// Ring buffer capacity in samples. At 48 kHz this is ~170 ms, at 16 kHz ~512 ms,
/// giving the UI enough history for a 100-150 ms time-domain waveform window.
const SPECTRUM_RING_SIZE: usize = 16384;

/// Update the global audio level store from the audio callback.
/// Uses try_lock so it never blocks the real-time audio thread.
pub fn update_audio_level_store(device_type: &DeviceType, rms: f32, peak: f32) {
    if let Ok(mut store) = AUDIO_LEVEL_STORE.try_lock() {
        match device_type {
            DeviceType::Microphone => {
                store.mic_rms = rms.min(1.0);
                store.mic_peak = peak.min(1.0);
                store.mic_active = rms > 0.0005;
            }
            DeviceType::System => {
                store.system_rms = rms.min(1.0);
                store.system_peak = peak.min(1.0);
                store.system_active = rms > 0.0005;
            }
        }
        store.last_update_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }
}

/// Read the current global audio levels.
pub fn get_audio_level_store() -> AudioLevelStore {
    AUDIO_LEVEL_STORE.lock().map(|s| s.clone()).unwrap_or_default()
}

/// Push the latest mono samples into the spectrum ring buffer.
/// Called from the real-time audio callback; uses try_lock to avoid blocking.
pub fn update_spectrum_ring(sample_rate: u32, samples: &[f32]) {
    if let Ok(mut ring) = SPECTRUM_RING.try_lock() {
        ring.extend(samples.iter().copied());
        while ring.len() > SPECTRUM_RING_SIZE {
            ring.pop_front();
        }
    }
    if let Ok(mut sr) = SPECTRUM_SAMPLE_RATE.try_lock() {
        *sr = sample_rate;
    }
}

/// Read the current spectrum samples and the sample rate they were captured at.
pub fn get_spectrum_data() -> (u32, Vec<f32>) {
    let samples = SPECTRUM_RING.lock().map(|r| r.iter().copied().collect()).unwrap_or_default();
    let sr = SPECTRUM_SAMPLE_RATE.lock().map(|s| *s).unwrap_or(48000);
    (sr, samples)
}

/// Device type for audio chunks
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    Microphone,
    System,
}

/// Audio chunk with metadata for processing
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub timestamp: f64,
    pub chunk_id: u64,
    pub device_type: DeviceType,
}

/// Processed audio chunk (post-VAD) for recording
#[derive(Debug, Clone)]
pub struct ProcessedAudioChunk {
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub timestamp: f64,
    pub device_type: DeviceType,
}

/// Comprehensive error types for audio system
#[derive(Debug, Clone)]
pub enum AudioError {
    DeviceDisconnected,
    StreamFailed,
    ProcessingFailed,
    TranscriptionFailed,
    ChannelClosed,
    InitializationFailed,
    ConfigurationError,
    PermissionDenied,
    BufferOverflow,
    SampleRateUnsupported,
}

impl AudioError {
    /// Check if error is recoverable (can attempt reconnection)
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Device disconnect is now recoverable - we can attempt reconnection
            AudioError::DeviceDisconnected => true,
            AudioError::StreamFailed => true,
            AudioError::ProcessingFailed => true,
            AudioError::TranscriptionFailed => true,
            AudioError::ChannelClosed => false,
            AudioError::InitializationFailed => false,
            AudioError::ConfigurationError => false,
            AudioError::PermissionDenied => false,
            AudioError::BufferOverflow => true,
            AudioError::SampleRateUnsupported => false,
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> &'static str {
        match self {
            AudioError::DeviceDisconnected => "Audio device was disconnected",
            AudioError::StreamFailed => "Audio stream encountered an error",
            AudioError::ProcessingFailed => "Audio processing failed",
            AudioError::TranscriptionFailed => "Speech transcription failed",
            AudioError::ChannelClosed => "Audio channel was closed unexpectedly",
            AudioError::InitializationFailed => "Failed to initialize audio system",
            AudioError::ConfigurationError => "Audio configuration error",
            AudioError::PermissionDenied => "Microphone permission denied",
            AudioError::BufferOverflow => "Audio buffer overflow",
            AudioError::SampleRateUnsupported => "Audio sample rate not supported",
        }
    }
}

/// Recording statistics
#[derive(Debug, Default)]
pub struct RecordingStats {
    pub chunks_processed: u64,
    pub total_duration: f64,
    pub last_activity: Option<Instant>,
}

/// Unified state management for audio recording
pub struct RecordingState {
    // Core recording state
    is_recording: AtomicBool,
    is_paused: AtomicBool,
    is_reconnecting: AtomicBool,  // NEW: Attempting to reconnect to device
    is_mic_muted: AtomicBool,     // Microphone mute (default: muted for meeting scenario)

    // Default-device follow state
    mic_stream_failed: AtomicBool,       // CPAL error on microphone stream
    system_stream_failed: AtomicBool,    // CPAL error on system audio stream
    current_default_mic: Mutex<Option<String>>, // Last known default microphone name
    current_default_sys: Mutex<Option<String>>, // Last known default system audio name
    waiting_for_device: AtomicBool,      // Microphone unavailable, waiting for it to return
    pending_device_check: AtomicBool,    // Resume/pause asked for an immediate device check
    rebuilding_streams: AtomicBool,      // Currently rebuilding streams, skip overlapping work

    // Audio devices
    microphone_device: Mutex<Option<Arc<AudioDevice>>>,
    system_device: Mutex<Option<Arc<AudioDevice>>>,
    // Track which device is disconnected for reconnection attempts
    disconnected_device: Mutex<Option<(Arc<AudioDevice>, DeviceType)>>,

    // Audio pipeline
    audio_sender: Mutex<Option<mpsc::UnboundedSender<AudioChunk>>>,

    // Memory optimization
    buffer_pool: AudioBufferPool,

    // Error handling
    error_count: AtomicU32,
    recoverable_error_count: AtomicU32,
    last_error: Mutex<Option<AudioError>>,
    error_callback: Mutex<Option<Box<dyn Fn(&AudioError) + Send + Sync>>>,

    // Statistics
    stats: Mutex<RecordingStats>,

    // Recording start time for accurate timestamps
    recording_start: Mutex<Option<Instant>>,
    // Pause time tracking
    pause_start: Mutex<Option<Instant>>,
    total_pause_duration: Mutex<std::time::Duration>,
}

impl RecordingState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            is_recording: AtomicBool::new(false),
            is_paused: AtomicBool::new(false),
            is_reconnecting: AtomicBool::new(false),
            is_mic_muted: AtomicBool::new(true), // Default muted for meeting scenario
            mic_stream_failed: AtomicBool::new(false),
            system_stream_failed: AtomicBool::new(false),
            current_default_mic: Mutex::new(None),
            current_default_sys: Mutex::new(None),
            waiting_for_device: AtomicBool::new(false),
            pending_device_check: AtomicBool::new(false),
            rebuilding_streams: AtomicBool::new(false),
            microphone_device: Mutex::new(None),
            system_device: Mutex::new(None),
            disconnected_device: Mutex::new(None),
            audio_sender: Mutex::new(None),
            buffer_pool: AudioBufferPool::new(16, 48000), // Pool of 16 buffers with 48kHz samples capacity
            error_count: AtomicU32::new(0),
            recoverable_error_count: AtomicU32::new(0),
            last_error: Mutex::new(None),
            error_callback: Mutex::new(None),
            stats: Mutex::new(RecordingStats::default()),
            recording_start: Mutex::new(None),
            pause_start: Mutex::new(None),
            total_pause_duration: Mutex::new(std::time::Duration::ZERO),
        })
    }

    // Recording control
    pub fn start_recording(&self) -> Result<()> {
        self.is_recording.store(true, Ordering::SeqCst);
        *self.recording_start.lock().unwrap() = Some(Instant::now());
        self.error_count.store(0, Ordering::SeqCst);
        self.recoverable_error_count.store(0, Ordering::SeqCst);
        *self.last_error.lock().unwrap() = None;
        Ok(())
    }

    pub fn stop_recording(&self) {
        self.is_recording.store(false, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        // Clear pause tracking when stopping
        *self.pause_start.lock().unwrap() = None;
        // CRITICAL: Clear audio sender to close the pipeline channel
        // This ensures the pipeline loop exits properly after processing all chunks
        *self.audio_sender.lock().unwrap() = None;
        // CRITICAL: Clear device references to release microphone/speaker
        // Without this, Arc<AudioDevice> references persist and keep the mic active
        *self.microphone_device.lock().unwrap() = None;
        *self.system_device.lock().unwrap() = None;
        *self.disconnected_device.lock().unwrap() = None;
        log::info!("Recording stopped, device references cleared");
    }

    pub fn pause_recording(&self) -> Result<()> {
        if !self.is_recording() {
            return Err(anyhow::anyhow!("Cannot pause when not recording"));
        }
        if self.is_paused() {
            return Err(anyhow::anyhow!("Recording is already paused"));
        }

        self.is_paused.store(true, Ordering::SeqCst);
        *self.pause_start.lock().unwrap() = Some(Instant::now());
        log::info!("Recording paused");
        Ok(())
    }

    pub fn resume_recording(&self) -> Result<()> {
        if !self.is_recording() {
            return Err(anyhow::anyhow!("Cannot resume when not recording"));
        }
        if !self.is_paused() {
            return Err(anyhow::anyhow!("Recording is not paused"));
        }

        // Calculate pause duration and add to total
        if let Some(pause_start) = self.pause_start.lock().unwrap().take() {
            let pause_duration = pause_start.elapsed();
            *self.total_pause_duration.lock().unwrap() += pause_duration;
            log::info!("Recording resumed after pause of {:.2}s", pause_duration.as_secs_f64());
        }

        self.is_paused.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    pub fn is_active(&self) -> bool {
        self.is_recording() && !self.is_paused()
    }

    // Microphone mute control
    pub fn mute_microphone(&self) {
        self.is_mic_muted.store(true, Ordering::SeqCst);
        log::info!("🔇 Microphone muted");
    }

    pub fn unmute_microphone(&self) {
        self.is_mic_muted.store(false, Ordering::SeqCst);
        log::info!("🎤 Microphone unmuted");
    }

    pub fn is_mic_muted(&self) -> bool {
        self.is_mic_muted.load(Ordering::SeqCst)
    }

    pub fn toggle_mic_mute(&self) -> bool {
        let prev = self.is_mic_muted.load(Ordering::SeqCst);
        let next = !prev;
        self.is_mic_muted.store(next, Ordering::SeqCst);
        log::info!("🔇 Microphone mute toggled: {} → {}", prev, next);
        next
    }

    // Reconnection state management
    pub fn start_reconnecting(&self, device: Arc<AudioDevice>, device_type: DeviceType) {
        self.is_reconnecting.store(true, Ordering::SeqCst);
        *self.disconnected_device.lock().unwrap() = Some((device, device_type));
        log::info!("Started reconnection attempt for device");
    }

    pub fn stop_reconnecting(&self) {
        self.is_reconnecting.store(false, Ordering::SeqCst);
        *self.disconnected_device.lock().unwrap() = None;
        log::info!("Stopped reconnection attempt");
    }

    pub fn is_reconnecting(&self) -> bool {
        self.is_reconnecting.load(Ordering::SeqCst)
    }

    pub fn get_disconnected_device(&self) -> Option<(Arc<AudioDevice>, DeviceType)> {
        self.disconnected_device.lock().unwrap().clone()
    }

    // Device management
    pub fn set_microphone_device(&self, device: Arc<AudioDevice>) {
        *self.microphone_device.lock().unwrap() = Some(device);
    }

    pub fn set_system_device(&self, device: Arc<AudioDevice>) {
        *self.system_device.lock().unwrap() = Some(device);
    }

    pub fn get_microphone_device(&self) -> Option<Arc<AudioDevice>> {
        self.microphone_device.lock().unwrap().clone()
    }

    pub fn get_system_device(&self) -> Option<Arc<AudioDevice>> {
        self.system_device.lock().unwrap().clone()
    }

    // Default-device follow state
    pub fn set_stream_failed(&self, device_type: DeviceType, failed: bool) {
        match device_type {
            DeviceType::Microphone => self.mic_stream_failed.store(failed, Ordering::SeqCst),
            DeviceType::System => self.system_stream_failed.store(failed, Ordering::SeqCst),
        }
    }

    pub fn is_stream_failed(&self, device_type: DeviceType) -> bool {
        match device_type {
            DeviceType::Microphone => self.mic_stream_failed.load(Ordering::SeqCst),
            DeviceType::System => self.system_stream_failed.load(Ordering::SeqCst),
        }
    }

    pub fn set_current_default_names(&self, mic: Option<String>, sys: Option<String>) {
        *self.current_default_mic.lock().unwrap() = mic;
        *self.current_default_sys.lock().unwrap() = sys;
    }

    pub fn get_current_default_names(&self) -> (Option<String>, Option<String>) {
        (
            self.current_default_mic.lock().unwrap().clone(),
            self.current_default_sys.lock().unwrap().clone(),
        )
    }

    pub fn set_waiting_for_device(&self, waiting: bool) {
        self.waiting_for_device.store(waiting, Ordering::SeqCst);
    }

    pub fn is_waiting_for_device(&self) -> bool {
        self.waiting_for_device.load(Ordering::SeqCst)
    }

    pub fn set_pending_device_check(&self, pending: bool) {
        self.pending_device_check.store(pending, Ordering::SeqCst);
    }

    pub fn is_pending_device_check(&self) -> bool {
        self.pending_device_check.load(Ordering::SeqCst)
    }

    pub fn set_rebuilding_streams(&self, rebuilding: bool) {
        self.rebuilding_streams.store(rebuilding, Ordering::SeqCst);
    }

    pub fn is_rebuilding_streams(&self) -> bool {
        self.rebuilding_streams.load(Ordering::SeqCst)
    }

    pub fn reset_recoverable_error_count(&self) {
        self.recoverable_error_count.store(0, Ordering::SeqCst);
    }

    // Audio pipeline management
    pub fn set_audio_sender(&self, sender: mpsc::UnboundedSender<AudioChunk>) {
        *self.audio_sender.lock().unwrap() = Some(sender);
    }

    pub fn send_audio_chunk(&self, chunk: AudioChunk) -> Result<()> {
        // Don't send audio chunks when paused
        if self.is_paused() {
            return Ok(()); // Silently discard chunks while paused
        }

        if let Some(sender) = self.audio_sender.lock().unwrap().as_ref() {
            sender.send(chunk).map_err(|_| anyhow::anyhow!("Failed to send audio chunk"))?;

            // Update statistics
            let mut stats = self.stats.lock().unwrap();
            stats.chunks_processed += 1;
            stats.last_activity = Some(Instant::now());
            Ok(())
        } else {
            // Return an error when no sender is available (pipeline not ready)
            Err(anyhow::anyhow!("Audio pipeline not ready - no sender available"))
        }
    }

    // Error handling
    pub fn set_error_callback<F>(&self, callback: F)
    where
        F: Fn(&AudioError) + Send + Sync + 'static,
    {
        *self.error_callback.lock().unwrap() = Some(Box::new(callback));
    }

    pub fn report_error(&self, error: AudioError) {
        let count = self.error_count.fetch_add(1, Ordering::SeqCst) + 1;

        // Track recoverable vs non-recoverable errors separately
        if error.is_recoverable() {
            let recoverable_count = self.recoverable_error_count.fetch_add(1, Ordering::SeqCst) + 1;
            log::warn!("Recoverable audio error ({}): {:?}", recoverable_count, error);

            // Allow more recoverable errors before stopping
            if recoverable_count >= 10 {
                log::error!("Too many recoverable errors ({}), stopping recording", recoverable_count);
                self.stop_recording();
            }
        } else {
            log::error!("Non-recoverable audio error: {:?}", error);
            // Stop immediately for non-recoverable errors
            self.stop_recording();
        }

        *self.last_error.lock().unwrap() = Some(error.clone());

        // Call error callback if set
        if let Some(callback) = self.error_callback.lock().unwrap().as_ref() {
            callback(&error);
        }

        // Fallback: stop recording after too many total errors
        if count >= 15 {
            log::error!("Too many total audio errors ({}), stopping recording", count);
            self.stop_recording();
        }
    }

    pub fn get_error_count(&self) -> u32 {
        self.error_count.load(Ordering::SeqCst)
    }

    pub fn get_recoverable_error_count(&self) -> u32 {
        self.recoverable_error_count.load(Ordering::SeqCst)
    }

    pub fn get_last_error(&self) -> Option<AudioError> {
        self.last_error.lock().unwrap().clone()
    }

    pub fn has_fatal_error(&self) -> bool {
        if let Some(error) = &*self.last_error.lock().unwrap() {
            !error.is_recoverable() && self.error_count.load(Ordering::SeqCst) > 0
        } else {
            false
        }
    }

    // Statistics
    pub fn get_stats(&self) -> RecordingStats {
        self.stats.lock().unwrap().clone()
    }

    pub fn get_recording_duration(&self) -> Option<f64> {
        self.recording_start
            .lock()
            .unwrap()
            .map(|start| start.elapsed().as_secs_f64())
    }

    pub fn get_active_recording_duration(&self) -> Option<f64> {
        self.recording_start.lock().unwrap().map(|start| {
            let total_duration = start.elapsed().as_secs_f64();
            let pause_duration = self.get_total_pause_duration();
            let current_pause = if self.is_paused() {
                self.pause_start
                    .lock()
                    .unwrap()
                    .map(|p| p.elapsed().as_secs_f64())
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            total_duration - pause_duration - current_pause
        })
    }

    pub fn get_total_pause_duration(&self) -> f64 {
        self.total_pause_duration.lock().unwrap().as_secs_f64()
    }

    pub fn get_current_pause_duration(&self) -> Option<f64> {
        if self.is_paused() {
            self.pause_start
                .lock()
                .unwrap()
                .map(|start| start.elapsed().as_secs_f64())
        } else {
            None
        }
    }

    // Memory management
    pub fn get_buffer_pool(&self) -> AudioBufferPool {
        self.buffer_pool.clone()
    }

    // Cleanup
    pub fn cleanup(&self) {
        self.stop_recording();
        self.stop_reconnecting();
        self.is_mic_muted.store(true, Ordering::SeqCst);
        self.mic_stream_failed.store(false, Ordering::SeqCst);
        self.system_stream_failed.store(false, Ordering::SeqCst);
        *self.current_default_mic.lock().unwrap() = None;
        *self.current_default_sys.lock().unwrap() = None;
        self.waiting_for_device.store(false, Ordering::SeqCst);
        self.pending_device_check.store(false, Ordering::SeqCst);
        self.rebuilding_streams.store(false, Ordering::SeqCst);
        *self.microphone_device.lock().unwrap() = None;
        *self.system_device.lock().unwrap() = None;
        *self.disconnected_device.lock().unwrap() = None;
        *self.audio_sender.lock().unwrap() = None;
        *self.last_error.lock().unwrap() = None;
        *self.error_callback.lock().unwrap() = None;
        *self.stats.lock().unwrap() = RecordingStats::default();
        *self.recording_start.lock().unwrap() = None;
        *self.pause_start.lock().unwrap() = None;
        *self.total_pause_duration.lock().unwrap() = std::time::Duration::ZERO;
        self.error_count.store(0, Ordering::SeqCst);
        self.recoverable_error_count.store(0, Ordering::SeqCst);

        // Clear buffer pool to free memory
        self.buffer_pool.clear();
    }
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            is_recording: AtomicBool::new(false),
            is_paused: AtomicBool::new(false),
            is_reconnecting: AtomicBool::new(false),
            is_mic_muted: AtomicBool::new(true),
            mic_stream_failed: AtomicBool::new(false),
            system_stream_failed: AtomicBool::new(false),
            current_default_mic: Mutex::new(None),
            current_default_sys: Mutex::new(None),
            waiting_for_device: AtomicBool::new(false),
            pending_device_check: AtomicBool::new(false),
            rebuilding_streams: AtomicBool::new(false),
            microphone_device: Mutex::new(None),
            system_device: Mutex::new(None),
            disconnected_device: Mutex::new(None),
            audio_sender: Mutex::new(None),
            buffer_pool: AudioBufferPool::new(16, 48000), // Pool of 16 buffers with 48kHz samples capacity
            error_count: AtomicU32::new(0),
            recoverable_error_count: AtomicU32::new(0),
            last_error: Mutex::new(None),
            error_callback: Mutex::new(None),
            stats: Mutex::new(RecordingStats::default()),
            recording_start: Mutex::new(None),
            pause_start: Mutex::new(None),
            total_pause_duration: Mutex::new(std::time::Duration::ZERO),
        }
    }
}

// Thread-safe cloning for RecordingStats
impl Clone for RecordingStats {
    fn clone(&self) -> Self {
        Self {
            chunks_processed: self.chunks_processed,
            total_duration: self.total_duration,
            last_activity: self.last_activity,
        }
    }
}