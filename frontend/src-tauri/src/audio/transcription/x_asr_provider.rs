// audio/transcription/x_asr_provider.rs
//
// X-ASR streaming transcription provider (Rust-native via sherpa-onnx OnlineRecognizer).
//
// 文本处理统一走 flow.rs 流式管线（停顿分段 + 翻译窗口 + 草稿翻译）：
// 识别器的累计文本直接推给管线，本文件不再做任何句界/分段决策
// （2026-09-21 重构：原 7 条句级提交规则全部由管线内的翻译窗口取代，
// 显示分段改由客户端静音检测驱动，与远程流式引擎行为一致）。

use super::provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use crate::audio::AudioChunk;
use crate::sherpa_onnx_engine::XAsrOnlineEngine;
use async_trait::async_trait;
use log::info;
use std::any::Any;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Runtime};

/// Global sequence counter（热切换对齐用；流式管线的单元序列号由 flow.rs 管理）。
static XASR_SEQUENCE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn reset_xasr_sequence_counter() {
    XASR_SEQUENCE_COUNTER.store(0, Ordering::SeqCst);
    info!("🔍 X-ASR sequence counter reset to 0");
}

/// 热切换对齐用：读/写当前序列号（与其它流式计数器统一到较大值，避免换引擎后撞 sequence_id）。
pub fn current_xasr_sequence() -> u64 {
    XASR_SEQUENCE_COUNTER.load(Ordering::SeqCst)
}

pub fn set_xasr_sequence(v: u64) {
    XASR_SEQUENCE_COUNTER.store(v, Ordering::SeqCst);
}

pub struct XAsrProvider {
    model_name: String,
    engine: Arc<XAsrOnlineEngine>,
}

impl XAsrProvider {
    pub fn new_with_engine(model_name: String, engine: Arc<XAsrOnlineEngine>) -> Self {
        Self { model_name, engine }
    }

    /// Transcribe a single audio file using the Rust-native OnlineRecognizer.
    /// Expects 16-bit PCM WAV (mono or stereo, any sample rate is resampled).
    pub async fn transcribe_file(&self, path: &std::path::Path) -> Result<String, String> {
        let file_bytes = tokio::fs::read(path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Parse minimal WAV header
        if file_bytes.len() < 44 {
            return Err("File too small to be a valid WAV".to_string());
        }
        let riff_marker = &file_bytes[0..4];
        if riff_marker != b"RIFF" {
            return Err("Not a valid WAV file".to_string());
        }
        let channels = u16::from_le_bytes([file_bytes[22], file_bytes[23]]);
        let sample_rate = u32::from_le_bytes([
            file_bytes[24],
            file_bytes[25],
            file_bytes[26],
            file_bytes[27],
        ]) as i32;
        let bits_per_sample = u16::from_le_bytes([file_bytes[34], file_bytes[35]]);

        if bits_per_sample != 16 {
            return Err(format!(
                "Only 16-bit WAV supported, got {} bits",
                bits_per_sample
            ));
        }

        let data_offset = 44u32; // skip standard header
        let raw_samples: Vec<i16> = file_bytes[data_offset as usize..]
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();

        if raw_samples.is_empty() {
            return Err("Empty audio file".to_string());
        }

        let samples: Vec<f32> = if channels == 1 {
            raw_samples.iter().map(|&s| s as f32 / 32768.0).collect()
        } else {
            // Downmix stereo to mono
            raw_samples
                .chunks_exact(channels as usize)
                .map(|ch| {
                    let sum: f32 = ch.iter().map(|&s| s as f32).sum();
                    sum / (ch.len() as f32 * 32768.0)
                })
                .collect()
        };

        if samples.is_empty() {
            return Err("Empty audio file".to_string());
        }

        let stream = self.engine.recognizer.create_stream();

        // Add 500ms silence padding for context
        let pad_len = (sample_rate as f32 * 0.5) as usize;
        let silence = vec![0.0f32; pad_len];
        stream.accept_waveform(sample_rate, &silence);
        while self.engine.recognizer.is_ready(&stream) {
            self.engine.recognizer.decode(&stream);
        }

        // Feed all samples in chunks to avoid huge single accept_waveform calls
        for chunk in samples.chunks(sample_rate as usize) {
            stream.accept_waveform(sample_rate, chunk);
            while self.engine.recognizer.is_ready(&stream) {
                self.engine.recognizer.decode(&stream);
            }
        }

        // Post-padding
        stream.accept_waveform(sample_rate, &silence);
        while self.engine.recognizer.is_ready(&stream) {
            self.engine.recognizer.decode(&stream);
        }

        stream.input_finished();
        while self.engine.recognizer.is_ready(&stream) {
            self.engine.recognizer.decode(&stream);
        }

        match self.engine.recognizer.get_result(&stream) {
            Some(result) => Ok(result.text),
            None => Ok(String::new()),
        }
    }

    /// Run continuous streaming transcription: 识别累计文本直接喂 flow 管线。
    pub async fn run_streaming<R: Runtime>(
        &self,
        mut receiver: tokio::sync::mpsc::UnboundedReceiver<AudioChunk>,
        app: AppHandle<R>,
    ) {
        info!(
            "🎙️ X-ASR streaming starting (Rust-native), model={}",
            self.model_name
        );

        let stream = self.engine.recognizer.create_stream();
        let mut flow = super::flow::FlowPipeline::new();

        const FINAL_WAIT_MS: u64 = 800;

        let mut input_finished = false;
        let mut final_wait_deadline: Option<Instant> = None;
        let mut last_decode_time = Instant::now();
        let mut channel_closed = false;

        loop {
            let poll_duration = if input_finished {
                Duration::from_millis(50)
            } else {
                Duration::from_millis(10)
            };

            tokio::select! {
                biased;
                chunk = receiver.recv(), if !channel_closed => {
                    match chunk {
                        Some(chunk) => {
                            let samples_16k = if chunk.sample_rate != 16000 {
                                crate::audio::audio_processing::resample_audio(
                                    &chunk.data, chunk.sample_rate, 16000,
                                )
                            } else {
                                chunk.data
                            };

                            stream.accept_waveform(16000, &samples_16k);
                            while self.engine.recognizer.is_ready(&stream) {
                                self.engine.recognizer.decode(&stream);
                            }
                            last_decode_time = Instant::now();

                            // 音频时钟 + RMS 有声检测（停顿分段/冲刷用）
                            if !samples_16k.is_empty() {
                                let rms = (samples_16k.iter().map(|x| x * x).sum::<f32>()
                                    / samples_16k.len() as f32).sqrt();
                                flow.note_audio(chunk.timestamp, rms);
                            }

                            if let Some(result) = self.engine.recognizer.get_result(&stream) {
                                if !result.text.is_empty() {
                                    // 本地 X-ASR：文本已定稿（true），但**它的段是 VAD 切的、不能当语义边界**（false）
                                    flow.push_text(&app, &result.text, true, false);
                                }
                            }
                        }
                        None => {
                            channel_closed = true;
                            if !input_finished {
                                input_finished = true;
                                final_wait_deadline = Some(
                                    Instant::now() + Duration::from_millis(FINAL_WAIT_MS),
                                );

                                stream.input_finished();
                                while self.engine.recognizer.is_ready(&stream) {
                                    self.engine.recognizer.decode(&stream);
                                }
                                if let Some(result) = self.engine.recognizer.get_result(&stream) {
                                    // 本地 X-ASR：文本已定稿（true），但**它的段是 VAD 切的、不能当语义边界**（false）
                                    flow.push_text(&app, &result.text, true, false);
                                }

                                info!("🎙️ X-ASR audio input finished, waiting for final results");
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(poll_duration) => {
                    // Poll recognizer — always poll after input finished to catch final results
                    let should_poll = input_finished || last_decode_time.elapsed() < Duration::from_secs(2);
                    if should_poll {
                        if let Some(result) = self.engine.recognizer.get_result(&stream) {
                            if !result.text.is_empty() {
                                // 本地 X-ASR：文本已定稿（true），但**它的段是 VAD 切的、不能当语义边界**（false）
                                    flow.push_text(&app, &result.text, true, false);
                            }
                        }
                    }

                    // 管线周期检查：停顿冲刷翻译、停顿分段、停滞兜底
                    flow.tick(&app);

                    if input_finished {
                        if let Some(deadline) = final_wait_deadline {
                            if Instant::now() >= deadline {
                                flow.finish(&app);
                                break;
                            }
                        }
                    }
                }
            }
        }

        info!("🎙️ X-ASR streaming ended (Rust-native)");
    }
}

#[async_trait]
impl TranscriptionProvider for XAsrProvider {
    async fn transcribe(
        &self,
        _audio: Vec<f32>,
        _language: Option<String>,
    ) -> Result<TranscriptResult, TranscriptionError> {
        Err(TranscriptionError::EngineFailed(
            "X-ASR uses streaming mode, not chunk-based transcribe()".into(),
        ))
    }

    async fn is_model_loaded(&self) -> bool {
        true
    }

    async fn get_current_model(&self) -> Option<String> {
        Some(self.model_name.clone())
    }

    fn provider_name(&self) -> &'static str {
        "x-asr"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
