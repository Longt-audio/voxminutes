#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use log::{LevelFilter, Log, Metadata, Record};

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
        Self { file: Mutex::new(file), level_filter: filter }
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
    use sysinfo::{System, RefreshKind};

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

    let cpus: Vec<String> = sys.cpus().iter().map(|c| c.brand().trim().to_string()).collect();
    let cpu_brand = cpus.first().cloned().unwrap_or_else(|| "unknown".to_string());
    let cpu_cores = sys.cpus().len();
    let cpu_vendor = sys.cpus().first().map(|c| c.vendor_id().trim().to_string()).unwrap_or_default();
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
            if leaf1.ecx & (1 << 20) != 0 { features.push("sse4.2"); }
            if leaf1.ecx & (1 << 28) != 0 { features.push("avx"); }
            if leaf1.ecx & (1 << 12) != 0 { features.push("fma"); }
            let max_leaf = __cpuid(0).eax;
            if max_leaf >= 7 {
                let leaf7 = __cpuid(7);
                if leaf7.ebx & (1 << 5)  != 0 { features.push("avx2"); }
                if leaf7.ebx & (1 << 16) != 0 { features.push("avx512f"); }
                if leaf7.ebx & (1 << 30) != 0 { features.push("avx512bw"); }
                if leaf7.ebx & (1 << 31) != 0 { features.push("avx512vl"); }
                if leaf7.ebx & (1 << 17) != 0 { features.push("avx512dq"); }
                if leaf7.ecx & (1 << 11) != 0 { features.push("avx512vnni"); }
            }
            let has512 = features.iter().any(|f| f.starts_with("avx512"));
            (features, has512)
        }
    };
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let (cpu_features, has_avx512) = (Vec::new(), false);

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

    let log_path = if let Some(app_dir) = resolve_app_data_dir() {
        let log_dir = app_dir.join("logs");
        let _ = fs::create_dir_all(&log_dir);
        // Retain the last 30 log files so recent history is available.
        cleanup_old_logs(&log_dir, 30);
        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
        log_dir.join(format!("app_{}.log", ts))
    } else {
        // Last-resort fallback: a log file in the current directory.
        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
        PathBuf::from(format!("app_{}.log", ts))
    };

    match fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(file) => {
            let sys_info = collect_system_info();
            write_log_header(&file, &sys_info, &log_path);

            // Expose the log file path via APP_LOG_FILE so child tooling can
            // append to the same unified log if needed.
            std::env::set_var("APP_LOG_FILE", log_path.as_os_str());

            let dual = DualLogger::new(file);
            log::set_max_level(dual.level_filter());
            let _ = log::set_boxed_logger(Box::new(dual));
        }
        Err(e) => {
            // Fallback: env_logger to stderr only
            eprintln!("[WARN] Could not open log file {:?}: {}", log_path, e);
            env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or("info"),
            )
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
        }
    }

    log::info!("Starting application...");
    log::info!("Unified log file: {:?}", log_path);
    app_lib::run();
}
