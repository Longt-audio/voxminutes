// task_session.rs
//
// 任务会话 id（2026-09-22 新增）：让网关能把「逐笔调用」归到「用户看得懂的任务」上。
//
// 背景：网关按次调用计费，一次 6 分钟录音会产生 1 笔识别 + 几百笔逐块翻译。
// 用户中心的积分明细以前就是这堆流水，看不出「这次录音一共花了多少」。
// 网关侧已支持按任务聚合（见 gateway/src/attribution.ts），但**任务边界只有客户端知道**：
// 「第 3 次录音」结束到「第 4 次录音」开始之间没有任何网关可见的信号。
//
// 方案：客户端每次开始一个任务（录音 / 离线重识别）时生成一个 id，
// 并在该任务期间的所有远程请求上带 `x-vox-session: <id>`（HTTP 头或 WebSocket 握手头）。
// 网关据此把同一 id 的调用聚成一个任务；老客户端不带时网关按静默间隔推断。
//
// 实现要点：只保留**一个**「当前任务」槽位。
//   远程调用（翻译、流式识别、离线识别）散落在各处，不可能逐层传参；
//   而同一时刻只会有一个任务在跑（录音中 / 重识别中 / 总结中），
//   所以「最近开始的那个任务」就是当前任务，读取时直接取槽位即可。
//   这样也自动处理了「录音过程中对旧录音做重识别」——重识别开始时接管槽位，与线上问题一致。
//
// ⚠️ 坑（2026-09-27 生产实证）：CURRENT 在任务结束后**从不清空**——刻意的，
//   录音结束后的会议总结要归到刚结束的任务里（这个归组在生产数据里可读性很好）。
//   副作用是：任何「不属于任何任务」的后台探活（如下一次录音前的流式通道预检
//   probe_streaming_channel）若也走 current_session()，会带上**上一次**录音的旧
//   rec- id，网关把这个 0 字节秒关的预检会话计入上一个任务（实测 rec-xxx 的
//   ASR 任务行 calls=2，多出来的那次就是下一次的预检）。
//   所以预检必须走 probe_session()（一次性 id、不看不写 CURRENT），不要图省事
//   复用 current_session()。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// 任务类型 → 前缀（网关只做展示，不做语义判断）。
const PREFIX_RECORDING: &str = "rec";
const PREFIX_RETRANSCRIBE: &str = "re";

static SEQ: AtomicU64 = AtomicU64::new(0);

/// 当前任务 id（None = 还没开始过任何任务）。
static CURRENT: Mutex<Option<String>> = Mutex::new(None);

/// 测试串行锁（模块级，两个 #[cfg(test)] 模块共用）。
/// 这些用例都会读写全局 `CURRENT` / `SEQ`，而 cargo test 默认多线程并行 ——
/// 各模块各持一把锁起不到互斥作用（实测 `unstarted_slot_falls_back_to_process_id`
/// 会因为别的用例刚设了 CURRENT 而假失败）。
#[cfg(test)]
pub(crate) static TEST_GUARD: Mutex<()> = Mutex::new(());

/// 当前毫秒时间戳（2026-09-23 修正：原名 boot_ms 但返回的是「此刻」而不是启动时刻 —
/// 名实不符导致「进程级兜底 id 稳定」的意图从未真正实现，见 current_session）。
fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn start(prefix: &str) -> String {
    let seq = SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let id = format!("{prefix}-{:x}-{seq}", now_ms());
    if let Ok(mut cur) = CURRENT.lock() {
        *cur = Some(id.clone());
    }
    id
}

/// 开始一次录音任务。
pub fn start_recording_session() -> String {
    start(PREFIX_RECORDING)
}

/// 开始一次离线重识别任务。
pub fn start_retranscribe_session() -> String {
    start(PREFIX_RETRANSCRIBE)
}

/// 一次性「单发动作」id（proc-<毫秒>-<序号>，每次调用都不同）：不看不写 CURRENT。
/// current_session 的无任务兜底与 probe_session 共用这条。
fn one_off_id() -> String {
    let seq = SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    format!("proc-{:x}-{seq}", now_ms())
}

/// 当前任务的会话 id：请求头里带的就是它。
/// 还没开始过任何任务时返回**一次性**的兜底 id（proc-<时间>-<序号>，每次调用都不同）：
/// 刻意的——兜底路径上的动作是「翻译页点一次翻译」「TTS 页点一次合成」这类单发动作，
/// 每次动作在网关任务统计里各自成行，正是用户想看的粒度；若返回进程级稳定 id，
/// 一整天互不相干的翻译/合成会被聚成同一个跨小时的假任务。
/// 不返回空串，避免调用方写出「有时带头、有时不带」的分支（网关侧两种行为都要能正确处理）。
pub fn current_session() -> String {
    if let Ok(cur) = CURRENT.lock() {
        if let Some(id) = cur.as_ref() {
            return id.clone();
        }
    }
    one_off_id()
}

/// 预检（probe）专用的一次性会话 id：**不看也不写 CURRENT**。
/// 为什么需要（2026-09-27 生产实证）：CURRENT 在任务结束后从不清空，于是
/// 「第二次录音前的流式通道预检」走 current_session() 会带上上一次录音的旧
/// rec- id，网关把这次 0 字节秒关的预检会话计入了上一个任务。预检不属于任何
/// 任务，必须在网关统计里各自成行（proc- 前缀）。
pub fn probe_session() -> String {
    one_off_id()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试串行锁（模块级共享）。见 TEST_GUARD 的注释。
    fn lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn reset() {
        if let Ok(mut cur) = CURRENT.lock() {
            *cur = None;
        }
    }

    #[test]
    fn session_id_is_stable_within_a_task() {
        let _g = lock();
        reset();
        let a = start_recording_session();
        assert_eq!(current_session(), a, "同一任务期间 id 必须稳定");
        assert_eq!(current_session(), a);
    }

    #[test]
    fn new_task_takes_over_the_slot() {
        let _g = lock();
        reset();
        let a = start_recording_session();
        assert_eq!(current_session(), a);
        let b = start_retranscribe_session();
        assert_ne!(a, b);
        assert_eq!(current_session(), b, "新任务开始时接管当前槽位");
    }

    #[test]
    fn retranscribe_takes_over_during_recording() {
        let _g = lock();
        reset();
        // 「录音中重识别旧录音」：后开始的离线重识别必须成为当前任务，
        // 否则它的批量识别会被算进正在进行的这次录音。
        let rec = start_recording_session();
        let re = start_retranscribe_session();
        assert_ne!(rec, re);
        assert_eq!(current_session(), re);
        // 再开一次录音应重新接管（重识别结束后继续录音的场景）
        let rec2 = start_recording_session();
        assert_eq!(current_session(), rec2);
        assert_ne!(rec2, rec);
    }

    #[test]
    fn unstarted_slot_falls_back_to_per_call_id() {
        let _g = lock();
        reset();
        let id = current_session();
        assert!(
            id.starts_with("proc-"),
            "未开始任务时应返回一次性兜底 id: {id}"
        );
        // 兜底 id 每次调用都不同：单发动作（翻译页一次翻译、TTS 一次合成）
        // 各自成任务，互不聚合（见 current_session 注释）
        assert_ne!(current_session(), id);
    }

    #[test]
    fn probe_session_never_touches_current() {
        let _g = lock();
        reset();
        // 有任务在进行时：预检绝不能拿到任务的 rec- id（否则预检被网关计入该任务）
        let rec = start_recording_session();
        let p1 = probe_session();
        assert!(p1.starts_with("proc-"), "预检 id 应是一次性 proc- id: {p1}");
        assert_ne!(p1, rec);
        assert_eq!(current_session(), rec, "预检不得改写/读取当前任务槽位");
        // 每次预检 id 都不同（各自成行，互不聚合）
        assert_ne!(probe_session(), p1);
        // 没有任务时同样是一次性 proc- id
        reset();
        assert!(probe_session().starts_with("proc-"));
    }

    #[test]
    fn ids_are_unique_and_header_safe() {
        let _g = lock();
        reset();
        // 网关按 [A-Za-z0-9._-] 清洗且截断到 64 字符；生成的 id 必须原样通过，
        // 否则同一任务的 id 被清洗后可能与另一个任务撞车。
        let mut seen = std::collections::HashSet::new();
        for _ in 0..500 {
            let id = start_recording_session();
            assert!(seen.insert(id.clone()), "id 重复: {id}");
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'),
                "id 含非法字符: {id}"
            );
            assert!(id.len() <= 64, "id 过长: {id}");
        }
    }
}

/// 为一次**逻辑请求**生成 id（放 `x-vox-request-id`）。
///
/// 用途与 session 不同：session 标识「哪一次任务」，request id 标识「哪一次调用」。
/// 网关按 (用户, request id) 去重：客户端超时重发时若复用同一个 id，网关直接返回首次结果
/// 且**不再扣费**（2026-09-22 生产实测过重复扣费：同模型同秒数十几秒内 2~3 笔，
/// 因为 idempotency 表建好了却从来没有写入方）。
///
/// ⚠️ 每次 HTTP 调用都要生成**新的** id：同一任务的多个 chunk 是各自独立的调用，
/// 共用 id 会被网关误判为重发而少扣费（资损方向相反的错误）。
/// 未来若加入「同一次调用自动重试」，必须在重试时复用同一个 id —— 所以这个函数
/// 设计成在 HTTP 请求构造处调用一次、把返回值捕获进闭包。
pub fn new_request_id() -> String {
    let seq = SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    format!("rq-{:x}-{seq}", now_ms())
}

#[cfg(test)]
mod request_id_tests {
    use super::*;

    fn reset_current() {
        if let Ok(mut cur) = CURRENT.lock() {
            *cur = None;
        }
    }

    #[test]
    fn request_ids_are_unique_and_header_safe() {
        let _g = TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let mut seen = std::collections::HashSet::new();
        for _ in 0..500 {
            let id = new_request_id();
            assert!(seen.insert(id.clone()), "request id 重复: {id}");
            assert!(
                id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'),
                "request id 含非法字符: {id}"
            );
            assert!(id.len() <= 64, "request id 过长: {id}");
        }
    }

    #[test]
    fn request_id_differs_from_session_id() {
        // 两者混用会导致「同一任务的多个 chunk 被当成重发」→ 少扣费
        let _g = TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        reset_current();
        let s = start_recording_session();
        let r = new_request_id();
        assert_ne!(s, r);
        assert!(r.starts_with("rq-"), "{r}");
    }
}
