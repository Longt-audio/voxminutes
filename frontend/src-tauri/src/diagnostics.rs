//! 诊断日志收集（2026-09-25）
//!
//! 解决什么：用户报障时我们拿不到他们机器上的 `logs/app_*.log`（软件已发布，
//! 不可能让每个用户去找目录、发文件），而他们口述的现象往往缺关键信息
//! （WebSocket 关闭码、重连次数、看门狗判定、上游返回的错误码）。
//!
//! 做法：用户勾选「附加诊断日志」时，取**最近日志文件的尾部**（报障要看的是
//! 最后发生了什么，不是启动信息），做脱敏后随反馈一起上传到网关。
//!
//! 三条设计约束：
//!   ① **脱敏**：授权码、API key、Bearer token、系统用户名一律抹掉；
//!   ② **有界**：按文件数模式每文件 ≤160KB，按时间窗（最近一天）模式每文件 ≤80KB
//!      （文件多，总量要收住），不因为用户日志写了几个月就传一个 G；
//!   ③ **不失败**：日志收集失败绝不能挡住反馈提交（返回说明文本，调用方照发文字）。

use std::fs;
use std::path::{Path, PathBuf};

/// 按文件数模式：每个文件最多取尾部多少字节
const MAX_TAIL_BYTES: usize = 160_000;
/// 按时间窗模式（最近一天）：单文件上限收紧，防止一天内多次重启把总量撑爆
const MAX_TAIL_BYTES_TIME_WINDOW: usize = 80_000;
/// 默认回溯几个日志文件（跨重启的上下文：上一个文件尾部 + 当前文件）
const DEFAULT_MAX_FILES: usize = 2;
/// 按时间窗模式下的文件数硬上限（日志目录最多保留 30 个文件，见 main.rs cleanup_old_logs）
const MAX_FILES_TIME_WINDOW: usize = 20;

/// 用户选择的附加范围（反馈表单下拉框）。按文件数（最近 N 次运行）或按时间窗（最近 N 小时）。
#[derive(Debug, Clone, Copy)]
pub enum DiagRange {
    /// 最近 N 个日志文件（≈ 最近 N 次运行）；每文件 ≤160KB
    LastFiles(usize),
    /// 最近 N 小时内修改过的日志文件；每文件 ≤80KB，总数 ≤20
    LastHours(u32),
}

impl DiagRange {
    /// 头部「范围」行的中文描述（网关后台同学直接读这段判断上下文跨度）
    fn describe(&self) -> String {
        match self {
            DiagRange::LastFiles(n) => format!("最近 {} 次运行（按文件数，每文件 ≤160KB）", n),
            DiagRange::LastHours(h) => format!("最近 {} 小时（按时间窗，每文件 ≤80KB）", h),
        }
    }
}

/// 收集诊断日志文本。永不返回 Err —— 拿不到日志时返回一段说明，反馈照常能发。
///
/// `max_files` / `since_hours` 二选一（都给了以时间窗为准）；都不给 = 默认最近 2 个文件，
/// 保持 2026-09-25 以来的旧行为。
pub fn collect_diag_log(max_files: Option<usize>, since_hours: Option<u32>) -> String {
    let range = match since_hours {
        Some(h) => DiagRange::LastHours(h),
        None => DiagRange::LastFiles(max_files.unwrap_or(DEFAULT_MAX_FILES).clamp(1, MAX_FILES_TIME_WINDOW)),
    };
    let files = recent_log_files(range);
    if files.is_empty() {
        return format!(
            "[diag] 未找到客户端日志文件（范围：{}；APP_LOG_FILE={:?}）\n",
            range.describe(),
            std::env::var("APP_LOG_FILE").ok()
        );
    }

    let per_file_cap = match range {
        DiagRange::LastFiles(_) => MAX_TAIL_BYTES,
        DiagRange::LastHours(_) => MAX_TAIL_BYTES_TIME_WINDOW,
    };

    let mut out = String::new();
    out.push_str(&format!(
        "==== VoxMinutes 诊断日志 ====\n版本: {}\n系统: {} {}\n时间: {}\n范围: {}\n文件数: {}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        range.describe(),
        files.len(),
    ));

    for (idx, path) in files.iter().enumerate() {
        let tail = read_tail(path, per_file_cap);
        match tail {
            Some(text) => {
                out.push_str(&format!(
                    "\n---- [{}] {} （尾部 {} 字节）----\n",
                    idx + 1,
                    path.display(),
                    text.len()
                ));
                out.push_str(&redact(&text));
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            None => out.push_str(&format!("\n---- [{}] {} 读取失败 ----\n", idx + 1, path.display())),
        }
    }
    out
}

/// 用户在反馈表单里手动挑选的日志文件（「附加日志文件…」按钮，对话框默认定位到 log_dir()）。
/// 与自动收集同一套脱敏与单文件上限，头部标明「用户手动附加」，便于网关侧区分来源。
/// 同样永不 panic/失败外传：读不了的文件记一行「读取失败」就跳过。
pub fn collect_manual_logs(paths: &[PathBuf]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "==== 用户手动附加的日志文件 ====\n时间: {}\n文件数: {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        paths.len(),
    ));
    for (idx, path) in paths.iter().enumerate() {
        match read_tail(path, MAX_TAIL_BYTES) {
            Some(text) => {
                out.push_str(&format!(
                    "\n---- [手动 {}] {} （尾部 {} 字节）----\n",
                    idx + 1,
                    path.display(),
                    text.len()
                ));
                out.push_str(&redact(&text));
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            None => out.push_str(&format!("\n---- [手动 {}] {} 读取失败 ----\n", idx + 1, path.display())),
        }
    }
    out
}

/// 最近的日志文件（新的在前）：优先 `APP_LOG_FILE`（main.rs 启动时写入的环境变量，
/// 指向正在写的那个文件），否则退回「可执行文件同级 / 当前目录」的 logs/。
/// `range` 决定取多少个：按文件数取最近 N 个；按时间窗取最近 N 小时内修改过的（上限 20 个）。
fn recent_log_files(range: DiagRange) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("APP_LOG_FILE") {
        let path = PathBuf::from(p);
        if path.is_file() {
            out.push(path);
        }
    }

    let dir = log_dir();
    if let Some(dir) = dir {
        if let Ok(entries) = fs::read_dir(&dir) {
            let cutoff = match range {
                DiagRange::LastHours(h) => Some(std::time::SystemTime::now() - std::time::Duration::from_secs(h as u64 * 3600)),
                DiagRange::LastFiles(_) => None,
            };
            let mut logs: Vec<(std::time::SystemTime, PathBuf)> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "log").unwrap_or(false))
                .filter_map(|p| {
                    let m = fs::metadata(&p).ok()?.modified().ok()?;
                    Some((m, p))
                })
                // 时间窗模式：只要窗口内修改过的文件
                .filter(|(m, _)| cutoff.map(|c| m >= &c).unwrap_or(true))
                .collect();
            // 新的在前
            logs.sort_by(|a, b| b.0.cmp(&a.0));
            for (_, p) in logs {
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
    }

    let cap = match range {
        DiagRange::LastFiles(n) => n,
        DiagRange::LastHours(_) => MAX_FILES_TIME_WINDOW,
    };
    out.truncate(cap);
    out
}

/// 应用日志目录（反馈里「附加日志文件…」对话框的默认定位路径用，与 recent_log_files 同一套定位逻辑）。
/// macOS / Windows 都走 `APP_LOG_FILE` 的父目录（main.rs 启动时指向真实日志目录）；
/// dev 模式下退回可执行文件旁（target 附近）或当前目录的 logs/。
pub fn log_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("APP_LOG_FILE") {
        if let Some(parent) = Path::new(&p).parent() {
            return Some(parent.to_path_buf());
        }
    }
    // 生产：日志放在可执行文件同级的 logs/（与 main.rs 的约定一致，用户能找到）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let logs = dir.join("logs");
            if logs.is_dir() {
                return Some(logs);
            }
        }
    }
    let cwd_logs = PathBuf::from("logs");
    if cwd_logs.is_dir() {
        return Some(cwd_logs);
    }
    None
}

/// 读文件尾部；不足则整读。按 UTF-8 边界对齐，避免把多字节字符切一半。
fn read_tail(path: &Path, max_bytes: usize) -> Option<String> {
    let data = fs::read(path).ok()?;
    if data.len() <= max_bytes {
        return Some(String::from_utf8_lossy(&data).into_owned());
    }
    let mut start = data.len() - max_bytes;
    // 往后找到第一个 UTF-8 起始字节（0b10xxxxxx 是续字节，跳过）
    while start < data.len() && (data[start] & 0xC0) == 0x80 {
        start += 1;
    }
    Some(String::from_utf8_lossy(&data[start..]).into_owned())
}

/// 脱敏：授权码 / API key / Bearer token / 系统用户名。
///
/// 为什么必须做：日志里会打印「请求头/配置」这类上下文，直接把原文传上云等于
/// 把用户的授权码抄了一份走；用户名则是隐私。
pub fn redact(text: &str) -> String {
    let mut s = text.to_string();
    let patterns: &[(&str, &str)] = &[
        // 授权码（vx-xxxx）与常见 key 前缀
        (r"(?i)\bvx-[A-Za-z0-9_-]{6,}", "vx-***"),
        (r"(?i)\bsk-[A-Za-z0-9_-]{6,}", "sk-***"),
        (r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{6,}", "Bearer ***"),
        // 家目录里的用户名（Windows 与 macOS/Linux 两种写法）
        (r"(?i)([A-Za-z]:\\Users\\)[^\\/\s]+", "${1}<user>"),
        (r"(/Users/)[^/\s]+", "${1}<user>"),
        (r"(/home/)[^/\s]+", "${1}<user>"),
    ];
    for (pat, rep) in patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            s = re.replace_all(&s, *rep).into_owned();
        }
    }
    // 超长连续串（base64 音频/截图）压成占位符：日志里偶尔会带整段 base64，
    // 传上去既没用又占满上传体积。
    if let Ok(re) = regex::Regex::new(r"[A-Za-z0-9+/=]{200,}") {
        s = re.replace_all(&s, |caps: &regex::Captures| format!("[base64 {} chars]", caps[0].len())).into_owned();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_credentials() {
        let raw = "Authorization: Bearer vx-abcdef123456\napi_key=sk-1234567890abcdef\n";
        let out = redact(raw);
        assert!(!out.contains("abcdef123456"), "授权码没被抹掉: {out}");
        assert!(!out.contains("sk-1234567890abcdef"), "key 没被抹掉: {out}");
        // 顺序上 vx- 规则先命中，所以这里留下的是 "Bearer vx-***"（凭据已抹掉即可）
        assert!(out.contains("***"), "{out}");
    }

    #[test]
    fn redacts_bearer_when_no_prefix_rule_matches() {
        let out = redact("Authorization: Bearer abcdef1234567890");
        assert!(out.contains("Bearer ***"), "{out}");
        assert!(!out.contains("abcdef1234567890"), "{out}");
    }

    #[test]
    fn redacts_usernames_in_paths() {
        let out = redact(r"C:\Users\alice\AppData\logs\app.log and /Users/bob/logs/app.log and /home/carol/x.log");
        assert!(!out.contains("alice") && !out.contains("bob") && !out.contains("carol"), "{out}");
        assert!(out.contains("<user>"));
    }

    #[test]
    fn collapses_long_base64() {
        let blob = "A".repeat(500);
        let out = redact(&format!("audio={blob}"));
        // 具体长度会因贪婪匹配把前面的 '=' 也吃进去，所以只断言「被折叠」这个事实
        assert!(out.contains("[base64 "), "{out}");
        assert!(out.len() < 100, "没折叠：len={}", out.len());
    }

    #[test]
    fn tail_respects_utf8_boundary() {
        // 中文日志 + 很小的上限：不能切出乱码（from_utf8_lossy 会把半个字符变 �）
        let mut p = std::env::temp_dir();
        p.push(format!("vox_diag_test_{}.log", std::process::id()));
        fs::write(&p, "中文日志行一\n中文日志行二\n中文日志行三\n").unwrap();
        let tail = read_tail(&p, 20).unwrap();
        assert!(!tail.contains('\u{FFFD}'), "切出了半个字符: {tail:?}");
        let _ = fs::remove_file(&p);
    }
}
