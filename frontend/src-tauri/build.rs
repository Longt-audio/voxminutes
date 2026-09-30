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

    // Windows: 把原生依赖产出的运行时 DLL 搬进 resources-dll/，供 bundle.resources 打进安装包。
    stage_windows_runtime_dlls();

    tauri_build::build()
}

/// `bundle.resources`（见 `tauri.windows.conf.json`）里声明、Windows 上必须真的存在的
/// 运行时 DLL。DirectML.dll 故意不在列表里 —— 我们没有启用任何 DirectML 执行提供程序，
/// 之前那份 0 字节占位符纯属噪声。
const REQUIRED_WINDOWS_DLLS: &[&str] = &[
    "sherpa-onnx-c-api.dll",
    "sherpa-onnx-cxx-api.dll",
    "onnxruntime.dll",
    "onnxruntime_providers_shared.dll",
];

/// 把 `target/<triple>/<profile>/` 下的运行时 DLL 复制到 `src-tauri/resources-dll/`。
///
/// 为什么必须做这一步：
/// - `sherpa-onnx`（shared 模式）的构建脚本会把 sherpa-onnx-c-api.dll /
///   sherpa-onnx-cxx-api.dll / onnxruntime.dll / onnxruntime_providers_shared.dll
///   复制到 **可执行文件旁边**（`target/<triple>/<profile>/`）；
/// - `ort`（默认特性 download-binaries + copy-dylibs）同样把 onnxruntime.dll 放在那里；
/// - 但 **Tauri 打包器只收集 bundle.resources / externalBin 里声明的文件**，
///   可执行文件旁边的 DLL 不会自动进安装包。
/// 之前的做法是手工把 DLL 拷进 `resources-dll/`，而仓库里长期放着几个 **0 字节占位符**
/// （目录被 gitignore，新克隆的机器上更是空的）→ Windows 安装包要么根本打不出来、
/// 要么带着 0 字节 DLL 出厂，用户侧表现为「本地语音识别 / OPUS-MT 翻译加载失败」。
/// 这里改成构建时自动搬运，并在仍然缺失时用 cargo:warning 明确喊出来。
fn stage_windows_runtime_dlls() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR").map(std::path::PathBuf::from)
    else {
        return;
    };
    // OUT_DIR = target/<triple>/<profile>/build/<pkg>-<hash>/out → target/<triple>/<profile>
    let Some(profile_dir) = std::env::var("OUT_DIR").ok().and_then(|out| {
        std::path::Path::new(&out)
            .ancestors()
            .nth(3)
            .map(std::path::Path::to_path_buf)
    }) else {
        return;
    };

    let dest_dir = manifest_dir.join("resources-dll");
    if let Err(e) = std::fs::create_dir_all(&dest_dir) {
        println!(
            "cargo:warning=无法创建 {}: {}（Windows 打包会缺少运行时 DLL）",
            dest_dir.display(),
            e
        );
        return;
    }

    let Ok(entries) = std::fs::read_dir(&profile_dir) else {
        println!(
            "cargo:warning=无法读取 {}（Windows 打包会缺少运行时 DLL）",
            profile_dir.display()
        );
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let is_dll = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("dll"))
            .unwrap_or(false);
        if !is_dll || !path.is_file() {
            continue;
        }
        let src_len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if src_len == 0 {
            continue;
        }
        let Some(name) = path.file_name() else { continue };
        let dest = dest_dir.join(name);
        let need_copy = std::fs::metadata(&dest)
            .map(|m| m.len() != src_len)
            .unwrap_or(true);
        if !need_copy {
            continue;
        }
        match std::fs::copy(&path, &dest) {
            Ok(_) => println!(
                "cargo:warning=已暂存 Windows 运行时 DLL: {} ({} 字节)",
                dest.display(),
                src_len
            ),
            Err(e) => println!(
                "cargo:warning=暂存 {} 失败: {}（安装包会缺少该 DLL）",
                dest.display(),
                e
            ),
        }
    }

    for name in REQUIRED_WINDOWS_DLLS {
        let p = dest_dir.join(name);
        let len = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        if len == 0 {
            println!(
                "cargo:warning=❌ 缺少/空的 Windows 运行时 DLL: {} —— 打出来的包在 Windows 上本地语音识别与 OPUS-MT 翻译会加载失败。请确认 sherpa-onnx(shared) 与 ort 的构建脚本已成功下载（国内网络下常见失败）。",
                p.display()
            );
        }
    }
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
            println!(
                "cargo:warning=💡 Sherpa-ONNX (SenseVoice) runs on CPU with optimized performance"
            );
        }
        "linux" => {
            println!("cargo:warning=⚠️  Linux: Using CPU-only mode");
            println!(
                "cargo:warning=💡 Sherpa-ONNX (SenseVoice) runs on CPU with optimized performance"
            );
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
    let Ok(out_dir) = std::env::var("OUT_DIR") else {
        return;
    };
    // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out → target/<profile>
    let profile_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .map(|p| p.to_path_buf());
    let Some(dir) = profile_dir else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
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
            _ => println!("cargo:warning=Failed to re-sign dylib: {}", path.display()),
        }
    }
}
