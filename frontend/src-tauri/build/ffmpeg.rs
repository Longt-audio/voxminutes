//! Build-time FFmpeg binary bundling hook.
//!
//! The runtime audio pipeline (`audio/ffmpeg.rs`, `audio/ffmpeg_mixer.rs`) uses
//! the `ffmpeg-sidecar` crate, which can locate an existing FFmpeg binary or
//! download one on first use. This build hook is reserved for bundling a
//! platform-specific FFmpeg binary into `frontend/src-tauri/binaries/ffmpeg`
//! (referenced by `tauri.conf.json` `bundle.externalBin`).
//!
//! Currently a no-op so the crate builds without network access during
//! `cargo check`; a real downloader will be wired up as part of the macOS
//! adaptation work.

/// Ensure the FFmpeg binary is available for bundling at build time.
pub fn ensure_ffmpeg_binary() {
    // Intentionally empty for now. See module docs.
}
