// audio/transcription/mod.rs
//
// Transcription module: Provider abstraction, engine management, and worker pool.

pub mod provider;
pub mod sherpa_onnx_provider;
pub mod remote_asr_provider;
pub mod x_asr_provider;
pub mod engine;
pub mod worker;

// Re-export commonly used types
pub use provider::{TranscriptionError, TranscriptionProvider, TranscriptResult};
pub use engine::{
    TranscriptionEngine,
    validate_transcription_model_ready,
    get_or_init_transcription_engine,
    set_remote_asr_config,
    get_remote_asr_endpoint,
    get_remote_asr_model,
    is_remote_asr_configured,
    load_remote_asr_config_from_disk,
};
pub use remote_asr_provider::{check_remote_asr_health, ChunkContext};
pub use worker::{
    start_transcription_task,
    reset_speech_detected_flag,
    TranscriptUpdate
};
