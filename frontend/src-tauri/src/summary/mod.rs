//! Meeting-summary backend: remote LLM API configuration, connectivity
//! checks, streaming generation (OpenAI-compatible and Anthropic protocols),
//! and per-recording summary markdown files stored next to the audio.

pub mod client;
pub mod config;
pub mod local;
pub mod storage;

pub use client::{
    summary_cancel, summary_generate, summary_list_models, summary_test_connection,
};
pub use config::{summary_get_config, summary_save_config, SummaryApiConfig};
pub use local::summary_local_generate;
pub use storage::{summary_load, summary_save};
