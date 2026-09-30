#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use log::{LevelFilter, Log, Metadata, Record};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Finds the project root by walking up 3 directories from the executable
/// (target/debug/voxminutes.exe → target/ → project root). Falls back to the
/// current directory when the executable layout does not match.
fn find_project_root() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Ok(canonical) = exe.canonicalize() {
            let mut path = canonical;
            for _ in 0..3 {
                path = path.parent()?.to_path_buf();
            }
            return Some(path);
        }
    }
    std::env::current_dir().ok()
}

/// Resolve the base directory where application data (logs, DB, etc.) should
/// live. In production we prefer the executable's parent directory so logs are
/// next to the installed app; in development we use the project root.
fn resolve_app_data_dir() -> Option<PathBuf> {
    // Dev mode: keep logs in project root for convenience.
    if cfg!(debug_assertions) {
        return find_project_root();
    }

    // Production: place logs next to the executable so users can find them
    // without hunting inside %LOCALAPPDATA%.
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|p| p.to_path_buf())
}

/// 候选日志根目录（按优先级）：
/// 1. 生产环境可执行文件所在目录（用户最容易找到）；
/// 2. 每用户数据目录（Windows: `%LOCALAPPDATA%`；macOS: `~/Library/Application Support`）。
///
/// 为什么需要第 2 条：Windows 上应用可能被装到 `C:\Program Files\...`（管理员为全机
/// 安装），普通用户对该目录**没有写权限** —— 此时 `create_dir_all(logs)` 与
/// `OpenOptions::open` 都失败，日志会静默退化成「只写 stderr」，而 GUI 子系统
/// （`windows_subsystem = "windows"`）没有 stderr，等于**完全没有日志**，
/// 「反馈时附带诊断日志」也随之失效。macOS 上把 .app 放在 /Applications 时同理。
fn log_dir_candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(dir) = resolve_app_data_dir() {
        out.push(dir);
    }
    if let Some(dir) = dirs::data_local_dir() {
        out.push(dir.join("VoxMinutes"));
    }
    out
}

/// 目录是否真的可写：写一个探测文件再删掉（只判断 exists 会被 ACL/只读挂载骗过）。
fn dir_is_writable(dir: &Path) -> bool {
    let probe = dir.join(".voxminutes_log_write_test");
    match fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// 选择本次运行的日志文件路径：逐个候选目录尝试，都不行才退回当前目录。
fn resolve_log_path() -> PathBuf {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    for base in log_dir_candidates() {
        let log_dir = base.join("logs");
        if fs::create_dir_all(&log_dir).is_err() || !dir_is_writable(&log_dir) {
            continue;
        }
        // Retain the last 30 log files so recent history is available.
        cleanup_old_logs(&log_dir, 30);
        return log_dir.join(format!("app_{}.log", ts));
    }
    // Last-resort fallback: a log file in the current directory.
    PathBuf::from(format!("app_{}.log", ts))
}

/// Logger that writes to both stderr (console) and a single shared log file.
/// All layers (Rust, Python backend, frontend webview) are directed into this
/// one file so a support bundle only needs a single log.
struct DualLogger {
    file: Mutex<fs::File>,
    level_filter: LevelFilter,
}

impl DualLogger {
    fn new(file: fs::File) -> Self {
        let filter = std::env::var("RUST_LOG")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(LevelFilter::Info);
        Self {
            file: Mutex::new(file),
            level_filter: filter,
        }
    }

    fn level_filter(&self) -> LevelFilter {
        self.level_filter
    }
}

impl Log for DualLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level_filter
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let module = record.module_path().unwrap_or("unknown");
        let file = record.file().unwrap_or("?");
        let line = record.line().unwrap_or(0);
        let line = format!(
            "[{}] [{}] [{} {}:{}] {}\n",
            ts,
            record.level(),
            module,
            file,
            line,
            record.args()
        );

        // Console (stderr, unbuffered for immediate display)
        let _ = std::io::stderr().write_all(line.as_bytes());
        let _ = std::io::stderr().flush();

        // File
        if let Ok(mut f) = self.file.lock() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
        if let Ok(mut f) = self.file.lock() {
            let _ = f.flush();
        }
    }
}

/// Collect a small set of system facts useful for support/debugging.
fn collect_system_info() -> serde_json::Value {
    use sysinfo::{RefreshKind, System};

    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(sysinfo::CpuRefreshKind::everything())
            .with_memory(sysinfo::MemoryRefreshKind::everything()),
    );
    // Give sysinfo a moment to read CPU info on some platforms.
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_specifics(
        RefreshKind::new()
            .with_cpu(sysinfo::CpuRefreshKind::everything())
            .with_memory(sysinfo::MemoryRefreshKind::everything()),
    );

    let cpus: Vec<String> = sys
        .cpus()
        .iter()
        .map(|c| c.brand().trim().to_string())
        .collect();
    let cpu_brand = cpus
        .first()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let cpu_cores = sys.cpus().len();
    let cpu_vendor = sys
        .cpus()
        .first()
        .map(|c| c.vendor_id().trim().to_string())
        .unwrap_or_default();
    let cpu_frequency_mhz = sys.cpus().first().map(|c| c.frequency()).unwrap_or(0);
    let total_memory_gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

    // Detect CPU instruction-set features via CPUID (works everywhere, no
    // compiler-version-dependent macro). These directly explain why native
    // (onnxruntime/sherpa) code paths crash on certain machines — record them
    // so logs are self-explanatory for diagnosis.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let (cpu_features, has_avx512) = {
        use std::arch::x86_64::__cpuid;
        unsafe {
            let mut features: Vec<&'static str> = Vec::new();
            let leaf1 = __cpuid(1);
            if leaf1.ecx & (1 << 20) != 0 {
                features.push("sse4.2");
            }
            if leaf1.ecx & (1 << 28) != 0 {
                features.push("avx");
            }
            if leaf1.ecx & (1 << 12) != 0 {
                features.push("fma");
            }
            let max_leaf = __cpuid(0).eax;
            if max_leaf >= 7 {
                let leaf7 = __cpuid(7);
                if leaf7.ebx & (1 << 5) != 0 {
                    features.push("avx2");
                }
                if leaf7.ebx & (1 << 16) != 0 {
                    features.push("avx512f");
                }
                if leaf7.ebx & (1 << 30) != 0 {
                    features.push("avx512bw");
                }
                if leaf7.ebx & (1 << 31) != 0 {
                    features.push("avx512vl");
                }
                if leaf7.ebx & (1 << 17) != 0 {
                    features.push("avx512dq");
                }
                if leaf7.ecx & (1 << 11) != 0 {
                    features.push("avx512vnni");
                }
            }
            let has512 = features.iter().any(|f| f.starts_with("avx512"));
            (features, has512)
        }
    };
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let (cpu_features, has_avx512) = (Vec::<&'static str>::new(), false);

    serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "family": std::env::consts::FAMILY,
        "app_version": env!("CARGO_PKG_VERSION"),
        "cpu_brand": cpu_brand,
        "cpu_vendor": cpu_vendor,
        "cpu_cores": cpu_cores,
        "cpu_frequency_mhz": cpu_frequency_mhz,
        "cpu_features": cpu_features,
        "has_avx512": has_avx512,
        "total_memory_gb": format!("{:.2}", total_memory_gb),
        "hostname": System::host_name().unwrap_or_else(|| "unknown".to_string()),
    })
}

/// Write a formatted header with system info into the log file and stderr.
fn write_log_header(file: &fs::File, info: &serde_json::Value, log_path: &Path) {
    let banner = format!(
        "\n========================================\n\
         Application log started at {}\n\
         Log file: {}\n\
         System info: {}\n\
         ========================================\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f %:z"),
        log_path.display(),
        serde_json::to_string_pretty(info).unwrap_or_else(|_| info.to_string())
    );

    let _ = std::io::stderr().write_all(banner.as_bytes());
    // The file is wrapped in Mutex inside DualLogger; write header through a
    // temporary borrow before handing ownership to the logger.
    let mut f = file;
    let _ = f.write_all(banner.as_bytes());
    let _ = f.flush();
}

/// Remove old log files, keeping only the most recent `keep_count` files in
/// the logs directory. This prevents the log folder from growing unbounded.
fn cleanup_old_logs(log_dir: &Path, keep_count: usize) {
    let mut entries: Vec<_> = match fs::read_dir(log_dir) {
        Ok(iter) => iter
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("log"))
                    .unwrap_or(false)
            })
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                let modified = meta.modified().ok()?;
                Some((modified, e.path()))
            })
            .collect(),
        Err(_) => return,
    };

    if entries.len() <= keep_count {
        return;
    }

    // Sort newest first.
    entries.sort_by(|a, b| b.0.cmp(&a.0));

    for (_, path) in entries.into_iter().skip(keep_count) {
        let _ = fs::remove_file(&path);
    }
}

#[cfg(target_os = "windows")]
extern "system" {
    fn SetConsoleCP(wCodePageID: u32) -> i32;
    fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
}

/// 安装 panic 钩子：把 panic 写进统一日志文件，并单独留一份 crash 文件。
///
/// 为什么必须有（2026-09-30 真机崩溃排查踩到）：
///   默认的 panic 钩子只往 **stderr** 打印。而本程序 release 构建带
///   `windows_subsystem = "windows"`（GUI 子系统，没有控制台）→ stderr 无处可去，
///   panic 信息**彻底消失**：用户看到的只是「双击后窗口闪一下就没了」，
///   日志里最后一条还是崩溃前那句无关的 INFO，完全无法定位。
///   装了这个钩子之后，panic 会以 `[ERROR] PANIC at 文件:行号: 消息` 落进同一个日志文件，
///   同时在 `logs/` 下单独写一份 `crash_<时间戳>.log`（用户只需发这一个文件）。
fn install_panic_hook() {
    use std::io::Write as _;

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "未知位置".to_string());
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "（无法解析的 panic 载荷）".to_string()
        };

        let line = format!("PANIC at {location}: {message}");
        // 1) 统一日志（用户反馈时附带的诊断日志会带上它）
        log::error!("{}", line);

        // 2) 单独一份 crash 文件，便于用户「只发这一个」
        if let Ok(log_file) = std::env::var("APP_LOG_FILE") {
            let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let crash = std::path::Path::new(&log_file)
                .with_file_name(format!("crash_{stamp}.log"));
            let _ = fs::write(
                &crash,
                format!(
                    "{line}\n\n版本: {}\n系统: {} {}\n日志: {log_file}\n",
                    env!("CARGO_PKG_VERSION"),
                    std::env::consts::OS,
                    std::env::consts::ARCH,
                ),
            );
            // 没有日志目录时至少尝试写到 stderr（开发期可见）
            let _ = writeln!(std::io::stderr(), "{line}");
        }

        // 3) 交给默认钩子（有控制台时仍能看到，并保留 RUST_BACKTRACE 行为）
        previous(info);
    }));
}

fn main() {
    #[cfg(target_os = "windows")]
    unsafe {
        // Use UTF-8 as the console code page so that paths containing Chinese,
        // Korean, or other non-ASCII characters are handled consistently by
        // child processes spawned from this application.
        SetConsoleCP(65001);
        SetConsoleOutputCP(65001);
    }

    // Allow RUST_LOG to override the default level, but default to info.
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    let log_path = resolve_log_path();

    match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(file) => {
            let sys_info = collect_system_info();
            write_log_header(&file, &sys_info, &log_path);

            // Expose the log file path via APP_LOG_FILE so child tooling can
            // append to the same unified log if needed.
            std::env::set_var("APP_LOG_FILE", log_path.as_os_str());

            let dual = DualLogger::new(file);
            log::set_max_level(dual.level_filter());
            let _ = log::set_boxed_logger(Box::new(dual));
            install_panic_hook();
        }
        Err(e) => {
            // Fallback: env_logger to stderr only
            eprintln!("[WARN] Could not open log file {:?}: {}", log_path, e);
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
                .format(|buf, record| {
                    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    writeln!(
                        buf,
                        "[{}] [{}] [{}] {}",
                        ts,
                        record.level(),
                        record.module_path().unwrap_or("unknown"),
                        record.args()
                    )
                })
                .init();
            install_panic_hook();
        }
    }

    log::info!("Starting application...");
    log::info!("Unified log file: {:?}", log_path);
    app_lib::run();
}
