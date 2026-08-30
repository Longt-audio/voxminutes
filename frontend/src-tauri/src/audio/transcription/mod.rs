// audio/transcription/mod.rs
//
// Transcription module: Provider abstraction, engine management, and worker pool.

pub mod provider;
pub mod sherpa_onnx_provider;
pub mod remote_asr_provider;
pub mod remote_asr_streaming_provider;
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
    set_remote_config,
    get_remote_asr_endpoint,
    get_remote_asr_model,
    get_remote_translate_model,
    get_remote_tts_model,
    get_remote_license,
    set_remote_models,
    set_remote_enabled,
    remote_enabled,
    remote_enabled_raw,
    remote_api_base,
    remote_health_url,
    is_remote_asr_configured,
    load_remote_asr_config_from_disk,
};
pub use remote_asr_provider::{check_remote_asr_health, ChunkContext};
pub use remote_asr_streaming_provider::RemoteAsrStreamingProvider;
pub use worker::{
    start_transcription_task,
    reset_speech_detected_flag,
    TranscriptUpdate
};
