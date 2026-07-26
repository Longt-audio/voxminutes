/// Application configuration constants
///
/// Centralized definitions for default models and settings.
/// Used across database initialization, import, and retranscription.

/// Default translation model.
pub const DEFAULT_TRANSLATION_MODEL: &str = "hy-mt2-1.8b";

/// Translation model filename (Q4_K_M is the recommended quantization for the 1.8B model)
/// Official GGUF repo: https://huggingface.co/tencent/HY-MT1.5-1.8B-GGUF
pub const TRANSLATION_MODEL_FILENAME: &str = "HY-MT1.5-1.8B-Q4_K_M.gguf";

/// Translation model download URL (HuggingFace — official Tencent Hy-MT2-1.8B-GGUF repo)
pub const TRANSLATION_MODEL_URL: &str = "https://huggingface.co/tencent/Hy-MT2-1.8B-GGUF/resolve/main/Hy-MT2-1.8B-Q4_K_M.gguf";

/// Translation model mirror download URL
pub const TRANSLATION_MODEL_MIRROR_URL: &str = "https://hf-mirror.com/tencent/Hy-MT2-1.8B-GGUF/resolve/main/Hy-MT2-1.8B-Q4_K_M.gguf";

// === Translation inference parameters (official Hy-MT2 for 1.8B) ===
// Reference: https://huggingface.co/tencent/Hy-MT2-1.8B
pub const TRANSLATION_TEMPERATURE: f32 = 0.7;
pub const TRANSLATION_TOP_K: i32 = 20;
pub const TRANSLATION_TOP_P: f32 = 0.6;
pub const TRANSLATION_REPEAT_PENALTY: f32 = 1.05;
pub const TRANSLATION_MAX_TOKENS: i32 = 4096;

// === TTS Model URLs ===

/// Supertonic 3 model archive (en+ko+29 languages, multi-speaker)
pub const TTS_SUPERTONIC_MODEL_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/sherpa-onnx-supertonic-3-tts-int8-2026-05-11.tar.bz2";

/// Supertonic 3 mirror URL (China mirror)
pub const TTS_SUPERTONIC_MODEL_MIRROR_URL: &str =
    "https://hf-mirror.com/csukuangfj2/sherpa-onnx-tts-models/resolve/main/sherpa-onnx-supertonic-3-tts-int8-2026-05-11.tar.bz2";
