use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use anyhow::Result;
use log::{error, info};
use serde::Serialize;
use realfft::{RealFftPlanner, RealToComplex};

/// Number of time-domain samples sent to the UI per update. This is independent
/// of the capture sample rate; the raw ring buffer is decimated down to this
/// count so the frontend always draws a fixed-width waveform.
const UI_SAMPLE_COUNT: usize = 300;

/// Default time window (ms) of audio to display in the waveform visualizer.
const WAVEFORM_WINDOW_MS: f32 = 300.0;

/// FFT window for the frequency-domain spectrum sent to the UI.
const FFT_SIZE: usize = 2048;
/// Number of log-spaced frequency bands drawn by the frontend spectrum.
const SPECTRUM_BANDS: usize = 48;
const SPECTRUM_MIN_HZ: f32 = 60.0;
const SPECTRUM_MAX_HZ: f32 = 16000.0;
/// Magnitudes at or below this dB level render as silence.
const SPECTRUM_FLOOR_DB: f32 = -60.0;

lazy_static::lazy_static! {
    static ref FFT_PLAN: Arc<dyn RealToComplex<f32>> = {
        let mut planner = RealFftPlanner::<f32>::new();
        planner.plan_fft_forward(FFT_SIZE)
    };
}

/// Compute a normalized (0..1) log-frequency-band magnitude spectrum from the
/// latest `FFT_SIZE` samples, suitable for a PotPlayer-style spectrum display.
fn compute_spectrum(all_samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if all_samples.is_empty() || sample_rate == 0 {
        return vec![0.0; SPECTRUM_BANDS];
    }

    // Latest FFT_SIZE samples, zero-padded at the front when short.
    let mut input = FFT_PLAN.make_input_vec();
    let take = FFT_SIZE.min(all_samples.len());
    input[FFT_SIZE - take..].copy_from_slice(&all_samples[all_samples.len() - take..]);

    // Hann window.
    let n = FFT_SIZE as f32;
    for (i, v) in input.iter_mut().enumerate() {
        *v *= 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n - 1.0)).cos());
    }

    let mut output = FFT_PLAN.make_output_vec();
    if FFT_PLAN.process(&mut input, &mut output).is_err() {
        return vec![0.0; SPECTRUM_BANDS];
    }

    let scale = 2.0 / FFT_SIZE as f32;
    let magnitudes: Vec<f32> = output.iter().map(|c| c.norm() * scale).collect();
    let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
    let max_hz = SPECTRUM_MAX_HZ.min(sample_rate as f32 / 2.0);
    let log_min = SPECTRUM_MIN_HZ.ln();
    let log_span = max_hz.ln() - log_min;

    (0..SPECTRUM_BANDS)
        .map(|b| {
            let f_lo = (log_min + log_span * b as f32 / SPECTRUM_BANDS as f32).exp();
            let f_hi = (log_min + log_span * (b + 1) as f32 / SPECTRUM_BANDS as f32).exp();
            let lo = ((f_lo / bin_hz) as usize).min(magnitudes.len());
            let hi = (((f_hi / bin_hz).ceil() as usize).max(lo + 1)).min(magnitudes.len());
            let count = hi.saturating_sub(lo).max(1);
            let avg = magnitudes[lo..hi].iter().copied().sum::<f32>() / count as f32;
            let db = 20.0 * (avg + 1e-9).log10();
            ((db - SPECTRUM_FLOOR_DB) / -SPECTRUM_FLOOR_DB).clamp(0.0, 1.0)
        })
        .collect()
}

#[derive(Debug, Serialize, Clone)]
pub struct AudioLevelData {
    pub device_name: String,
    pub device_type: String, // "input" or "output"
    pub rms_level: f32,     // RMS level (0.0 to 1.0)
    pub peak_level: f32,    // Peak level (0.0 to 1.0)
    pub is_active: bool,    // Whether audio is being detected
    pub spectrum: Vec<f32>, // Log-frequency-band magnitudes (0..1) for the spectrum visualizer.
    pub samples: Vec<f32>,  // Time-domain samples for the waveform visualizer.
}

#[derive(Debug, Serialize, Clone)]
pub struct AudioLevelUpdate {
    pub timestamp: u64,
    pub levels: Vec<AudioLevelData>,
}

// Simple global monitoring state
static IS_MONITORING: AtomicBool = AtomicBool::new(false);

/// Decimate a slice of samples down to `target` points by averaging bins.
/// Preserves the overall waveform shape and is cheap enough to run every 50 ms.
fn downsample_samples(samples: &[f32], target: usize) -> Vec<f32> {
    if samples.is_empty() || target == 0 {
        return Vec::new();
    }
    if samples.len() <= target {
        return samples.to_vec();
    }

    let step = samples.len() as f32 / target as f32;
    (0..target)
        .map(|i| {
            let start = (i as f32 * step) as usize;
            let end = (((i + 1) as f32 * step) as usize).min(samples.len());
            let count = (end - start).max(1);
            let sum: f32 = samples[start..end].iter().copied().sum();
            sum / count as f32
        })
        .collect()
}

/// Return the most recent samples from the captured ring buffer clipped to the
/// desired waveform time window.
fn waveform_samples_for_rate(sample_rate: u32, all_samples: &[f32]) -> Vec<f32> {
    let window_samples = ((sample_rate as f32 * WAVEFORM_WINDOW_MS) / 1000.0) as usize;
    let take = window_samples.min(all_samples.len());
    if take == 0 {
        return Vec::new();
    }
    // The ring buffer stores oldest-to-newest, so the latest samples are at the end.
    all_samples[all_samples.len() - take..].to_vec()
}

/// Start audio level monitoring for specified devices
/// Reads real audio levels from the global store updated by the audio pipeline.
pub async fn start_monitoring<R: Runtime>(
    app_handle: AppHandle<R>,
    device_names: Vec<String>,
) -> Result<()> {
    info!("Starting real audio level monitoring for devices: {:?}", device_names);

    // Stop any existing monitoring
    IS_MONITORING.store(false, Ordering::SeqCst);

    // Wait a bit for any existing tasks to stop
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Start new monitoring
    IS_MONITORING.store(true, Ordering::SeqCst);

    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        while IS_MONITORING.load(Ordering::SeqCst) {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            let store = super::recording_state::get_audio_level_store();
            let (sample_rate, spectrum_samples) = super::recording_state::get_spectrum_data();

            // Capture a fixed time window of the most recent samples and decimate
            // them to a UI-friendly count. The frontend draws these as a fixed
            // non-scrolling waveform that jumps up and down in place.
            let window_samples = waveform_samples_for_rate(sample_rate, &spectrum_samples);
            let ui_samples = downsample_samples(&window_samples, UI_SAMPLE_COUNT);

            // If data is stale (> 2 seconds old), treat as silent
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let is_stale = now.saturating_sub(store.last_update_ms) > 2000;

            // Frequency-domain spectrum from the full-rate ring buffer (shared
            // by all reported devices; the frontend picks the first non-empty).
            let spectrum = if is_stale {
                vec![0.0; SPECTRUM_BANDS]
            } else {
                compute_spectrum(&spectrum_samples, sample_rate)
            };

            let mut levels: Vec<AudioLevelData> = Vec::new();

            for name in &device_names {
                let is_mic = name.to_lowercase().contains("microphone") || name.to_lowercase().contains("mic");
                let (rms, peak, active) = if is_stale {
                    (0.0f32, 0.0f32, false)
                } else if is_mic {
                    (store.mic_rms, store.mic_peak, store.mic_active)
                } else {
                    (store.system_rms, store.system_peak, store.system_active)
                };

                levels.push(AudioLevelData {
                    device_name: name.clone(),
                    device_type: if is_mic { "input".to_string() } else { "output".to_string() },
                    rms_level: rms,
                    peak_level: peak,
                    is_active: active,
                    spectrum: spectrum.clone(),
                    samples: ui_samples.clone(),
                });
            }

            if !levels.is_empty() {
                let update = AudioLevelUpdate {
                    timestamp: now,
                    levels,
                };

                if let Err(e) = app_handle_clone.emit("audio-levels", &update) {
                    error!("Failed to emit audio levels: {}", e);
                    break;
                }
            }
        }

        info!("Audio level monitoring task ended");
    });

    Ok(())
}

/// Stop audio level monitoring
pub async fn stop_monitoring() -> Result<()> {
    info!("Stopping audio level monitoring");
    IS_MONITORING.store(false, Ordering::SeqCst);
    Ok(())
}

/// Check if currently monitoring
pub fn is_monitoring() -> bool {
    IS_MONITORING.load(Ordering::SeqCst)
}
