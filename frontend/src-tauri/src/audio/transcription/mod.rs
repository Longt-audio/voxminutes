// audio/transcription/mod.rs
//
// Transcription module: Provider abstraction, engine management, and worker pool.

pub mod engine;
pub mod flow;
pub mod provider;
pub mod remote_asr_provider;
pub mod remote_asr_streaming_provider;
pub mod sherpa_onnx_provider;
pub mod worker;
pub mod x_asr_provider;

// Re-export commonly used types
pub use engine::{
    apply_explicit_remote_asr_model, effective_remote_endpoint, get_or_init_transcription_engine,
    get_remote_asr_endpoint,
    get_remote_asr_model, get_remote_asr_offline_model, get_remote_license,
    get_remote_translate_model, get_remote_summary_model, get_remote_tts_model, is_remote_asr_configured,
    is_remote_asr_streaming, is_remote_asr_streaming_model, load_remote_asr_config_from_disk,
    mark_remote_config_loaded,
    remote_api_base, remote_enabled, remote_enabled_raw, remote_health_url, set_remote_asr_config,
    set_remote_config, set_remote_enabled, set_remote_endpoint, set_remote_license,
    set_remote_models, validate_transcription_model_ready, TranscriptionEngine,
    DEFAULT_REMOTE_ENDPOINT,
};
pub use provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};
pub use remote_asr_provider::{check_remote_asr_health, ChunkContext};
pub use remote_asr_streaming_provider::RemoteAsrStreamingProvider;
pub use worker::{
    reset_speech_detected_flag, start_transcription_task, start_transcription_task_with_engine,
    TranscriptUpdate,
};
