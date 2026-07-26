#[path = "build/ffmpeg.rs"]
mod ffmpeg;

fn main() {
    // GPU Acceleration Detection and Build Guidance
    detect_and_report_gpu_capabilities();

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=Cocoa");
        println!("cargo:rustc-link-lib=framework=Foundation");

        // Let the enhanced_macos crate handle its own Swift compilation
        // The swift-rs crate build will be handled in the enhanced_macos crate's build.rs
    }

    // Download and bundle FFmpeg binary at build-time
    ffmpeg::ensure_ffmpeg_binary();

    tauri_build::build()
}

/// Detects GPU acceleration capabilities and provides build guidance
fn detect_and_report_gpu_capabilities() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    println!("cargo:warning=🚀 Building VoxMinutes for: {}", target_os);

    match target_os.as_str() {
        "macos" => {
            println!("cargo:warning=✅ macOS: Sherpa-ONNX (SenseVoice) CPU-optimized");
        }
        "windows" => {
            println!("cargo:warning=⚠️  Windows: Using CPU-only mode");
            println!("cargo:warning=💡 Sherpa-ONNX (SenseVoice) runs on CPU with optimized performance");
        }
        "linux" => {
            println!("cargo:warning=⚠️  Linux: Using CPU-only mode");
            println!("cargo:warning=💡 Sherpa-ONNX (SenseVoice) runs on CPU with optimized performance");
        }
        _ => {
            println!("cargo:warning=ℹ️  Unknown platform: {}", target_os);
        }
    }
}
