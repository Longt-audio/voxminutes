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
        // sherpa-onnx 共享库被复制到可执行文件旁边（@rpath 引用）
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
        // sherpa-onnx-sys 把预编译 dylib 复制到 target/<profile>/，其官方签名
        // 在本机 dyld 下会被判定无效（CODESIGNING Invalid Page 直接 SIGKILL）。
        // 统一 ad-hoc 重签，保证开发期可启动。
        resign_sherpa_dylibs();
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

/// Re-sign sherpa-onnx/onnxruntime dylibs next to the executable with an
/// ad-hoc signature (macOS). The prebuilt binaries downloaded by
/// sherpa-onnx-sys are killed by dyld on some macOS versions otherwise.
#[cfg(target_os = "macos")]
fn resign_sherpa_dylibs() {
    let Ok(out_dir) = std::env::var("OUT_DIR") else { return };
    // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out → target/<profile>
    let profile_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .map(|p| p.to_path_buf());
    let Some(dir) = profile_dir else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_dylib = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "dylib")
            .unwrap_or(false);
        if !is_dylib {
            continue;
        }
        let status = std::process::Command::new("codesign")
            .args(["--force", "--sign", "-"])
            .arg(&path)
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("cargo:warning=Re-signed dylib: {}", path.display())
            }
            _ => println!(
                "cargo:warning=Failed to re-sign dylib: {}",
                path.display()
            ),
        }
    }
}
