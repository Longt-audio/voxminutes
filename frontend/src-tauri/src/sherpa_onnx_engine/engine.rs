use anyhow::{anyhow, Result};
use sherpa_onnx::{
    OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig,
    OnlineRecognizer, OnlineRecognizerConfig, OnlineTransducerModelConfig,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::win_short_path::to_short_path_string;

#[derive(Debug, Clone, PartialEq)]
pub enum AsrModelType {
    SenseVoice,
    XAsr,
}

/// Strip Windows `\\?\` verbatim prefix from a path.
/// `Path::canonicalize()` adds this prefix, which breaks sherpa-onnx C++ path joins using `/`.
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with(r"\\?\") {
        PathBuf::from(&s[4..])
    } else {
        path.to_path_buf()
    }
}

pub struct SherpaOnnxEngine {
    pub recognizer: Arc<Mutex<OfflineRecognizer>>,
    model_path: PathBuf,
    model_name: String,
    model_type: AsrModelType,
}

impl SherpaOnnxEngine {
    pub fn create_sense_voice(model_dir: &Path, model_name: &str) -> Result<Self> {
        let model_dir = strip_verbatim_prefix(model_dir);
        let model_file = model_dir.join("model.onnx");
        let tokens_file = model_dir.join("tokens.txt");
        if !model_file.exists() {
            return Err(anyhow!(
                "model.onnx not found at {}",
                model_file.display()
            ));
        }
        if !tokens_file.exists() {
            return Err(anyhow!(
                "tokens.txt not found at {}",
                tokens_file.display()
            ));
        }
        // 语言偏好：优先使用用户选择（zh/en/...），默认 auto 自动检测
        let language = crate::get_language_preference_internal()
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| "auto".to_string());
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.sense_voice = OfflineSenseVoiceModelConfig {
            model: Some(to_short_path_string(&model_file)),
            language: Some(language),
            use_itn: true,
        };
        config.model_config.tokens = Some(to_short_path_string(&tokens_file));
        config.model_config.num_threads = 2;
        config.model_config.provider = Some("cpu".into());
        let recognizer = OfflineRecognizer::create(&config)
            .ok_or_else(|| anyhow!("Failed to create OfflineRecognizer for SenseVoice"))?;
        log::info!(
            "SherpaOnnxEngine loaded: {} from {}",
            model_name,
            model_dir.display()
        );
        Ok(Self {
            recognizer: Arc::new(Mutex::new(recognizer)),
            model_path: model_dir.to_path_buf(),
            model_name: model_name.to_string(),
            model_type: AsrModelType::SenseVoice,
        })
    }

    pub fn new(model_dir: &Path, model_name: &str) -> Result<Self> {
        Self::create_sense_voice(model_dir, model_name)
    }

    pub fn get_model_name(&self) -> &str {
        &self.model_name
    }
    pub fn get_model_path(&self) -> &Path {
        &self.model_path
    }
    pub fn get_model_type(&self) -> AsrModelType {
        self.model_type.clone()
    }

    pub fn validate_model_dir(model_dir: &Path) -> bool {
        model_dir.join("model.onnx").exists() && model_dir.join("tokens.txt").exists()
    }
}

// ── X-ASR Online Streaming Engine ───────────────────────────────────────

pub struct XAsrOnlineEngine {
    pub recognizer: OnlineRecognizer,
    model_name: String,
    model_type: AsrModelType,
    sample_rate: i32,
}

impl XAsrOnlineEngine {
    /// Create an X-ASR streaming engine from a model directory containing:
    ///   encoder.onnx  decoder.onnx  joiner.onnx  tokens.txt  [bpe.model]
    pub fn create_x_asr(model_dir: &Path, model_name: &str) -> Result<Self> {
        let model_dir = strip_verbatim_prefix(model_dir);
        let encoder = model_dir.join("encoder.onnx");
        let decoder = model_dir.join("decoder.onnx");
        let joiner = model_dir.join("joiner.onnx");
        let tokens = model_dir.join("tokens.txt");

        for (name, p) in &[
            ("encoder.onnx", &encoder),
            ("decoder.onnx", &decoder),
            ("joiner.onnx", &joiner),
            ("tokens.txt", &tokens),
        ] {
            if !p.exists() {
                return Err(anyhow!("{} not found at {}", name, p.display()));
            }
        }

        let mut config = OnlineRecognizerConfig::default();
        config.model_config.transducer = OnlineTransducerModelConfig {
            encoder: Some(to_short_path_string(&encoder)),
            decoder: Some(to_short_path_string(&decoder)),
            joiner: Some(to_short_path_string(&joiner)),
        };
        config.model_config.tokens = Some(to_short_path_string(&tokens));
        config.model_config.num_threads = 4;
        config.model_config.provider = Some("cpu".into());
        config.model_config.model_type = Some("zipformer2".into());
        config.model_config.modeling_unit = Some("cjkchar".into());
        // NOTE: bpe_vocab is intentionally NOT set here.
        // The new model includes bpe.model, but setting it with modeling_unit="cjkchar"
        // can cause extra spacing between CJK characters. English output quality is
        // still acceptable without BPE (individual letters instead of subwords).
        config.decoding_method = Some("greedy_search".into());
        config.enable_endpoint = false;

        let recognizer = OnlineRecognizer::create(&config)
            .ok_or_else(|| anyhow!("Failed to create OnlineRecognizer for X-ASR"))?;

        log::info!(
            "XAsrOnlineEngine loaded: {} from {} ({} threads)",
            model_name,
            model_dir.display(),
            config.model_config.num_threads,
        );
        Ok(Self {
            recognizer,
            model_name: model_name.to_string(),
            model_type: AsrModelType::XAsr,
            sample_rate: 16000,
        })
    }

    pub fn get_model_name(&self) -> &str {
        &self.model_name
    }

    pub fn get_model_type(&self) -> AsrModelType {
        self.model_type.clone()
    }

    pub fn get_sample_rate(&self) -> i32 {
        self.sample_rate
    }

    /// Validate that a directory contains the required X-ASR model files.
    pub fn validate_model_dir(model_dir: &Path) -> bool {
        model_dir.join("encoder.onnx").exists()
            && model_dir.join("decoder.onnx").exists()
            && model_dir.join("joiner.onnx").exists()
            && model_dir.join("tokens.txt").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// 最小 PCM16 mono WAV 解析（RIFF fmt/data chunk）。
    fn read_wav_pcm16(path: &Path) -> Result<(Vec<f32>, u32), String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err("invalid wav".to_string());
        }
        let mut pos = 12usize;
        let mut rate = 0u32;
        let mut data = &[][..];
        while pos + 8 <= bytes.len() {
            let id = &bytes[pos..pos + 4];
            let size = u32::from_le_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]]) as usize;
            if id == b"fmt " {
                rate = u32::from_le_bytes([bytes[pos + 12], bytes[pos + 13], bytes[pos + 14], bytes[pos + 15]]);
            }
            if id == b"data" {
                data = &bytes[pos + 8..(pos + 8 + size).min(bytes.len())];
                break;
            }
            pos += 8 + size + (size % 2);
        }
        if rate == 0 || data.is_empty() {
            return Err("no fmt/data chunk".to_string());
        }
        let samples: Vec<f32> = data
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
            .collect();
        Ok((samples, rate))
    }

    /// X-ASR 解码速度基准：480ms 流式块喂 example_audio.wav，输出 RTF。
    /// 手动运行：cargo test -p voxminutes bench_x_asr -- --ignored --nocapture
    #[test]
    #[ignore]
    fn bench_x_asr_decode_speed() {
        let model_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../models/sherpa-onnx-x-asr-480ms-streaming-zipformer-transducer-zh-en-punct-2026-06-05",
        );
        if !model_dir.exists() {
            eprintln!("model dir missing, skipping");
            return;
        }
        let wav_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../example_audio.wav");
        let (samples, rate) = read_wav_pcm16(&wav_path).expect("read wav");
        let samples_16k = crate::audio::audio_processing::resample_audio(&samples, rate, 16000);
        let audio_secs = samples_16k.len() as f64 / 16000.0;

        let t0 = Instant::now();
        let engine = XAsrOnlineEngine::create_x_asr(&model_dir, "x-asr-480ms").expect("load engine");
        eprintln!("engine load time: {:?}", t0.elapsed());

        let stream = engine.recognizer.create_stream();
        let t1 = Instant::now();
        for chunk in samples_16k.chunks(7680) {
            stream.accept_waveform(16000, chunk);
            while engine.recognizer.is_ready(&stream) {
                engine.recognizer.decode(&stream);
            }
        }
        stream.input_finished();
        while engine.recognizer.is_ready(&stream) {
            engine.recognizer.decode(&stream);
        }
        let decode_elapsed = t1.elapsed();
        let rtf = decode_elapsed.as_secs_f64() / audio_secs;

        if let Some(result) = engine.recognizer.get_result(&stream) {
            eprintln!("text: {}", result.text.chars().take(120).collect::<String>());
        }
        eprintln!(
            "audio: {:.1}s | decode: {:?} | RTF = {:.2} ({:.0}% of real time)",
            audio_secs,
            decode_elapsed,
            rtf,
            rtf * 100.0
        );
        eprintln!(
            "结论: {}",
            if rtf < 0.5 {
                "解码远快于实时，延迟不是算力问题"
            } else if rtf < 1.0 {
                "解码能跟上但余量小，争抢下会落后"
            } else {
                "解码跟不上实时，会持续积压"
            }
        );
    }
}
