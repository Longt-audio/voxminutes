// translation/remote.rs
//
// 远程翻译：走开发者网关的 OpenAI 兼容 chat completions（SSE 流式）。
// 复用 llm.rs 的 build_prompt（纠错翻译指令）与 postprocess（去回声清洗），
// 只把底层生成从 llama-helper 换成 HTTP。
//
// 超时策略（2026-09-24 补）：
//   · `.timeout(120s)` 是**整个请求**的上限（含流式读完），单靠它挡不住假死 ——
//     2026-09-23 那次跨境下行 stall 里，翻译 SSE 和 ASR WS 是**同时**被卡住的，
//     ASR 侧加了看门狗，翻译侧如果只留 120s 总超时，字幕 15s 恢复了、译文还能再卡两分钟。
//   · 所以这里给「相邻两块数据之间」加 IDLE_TIMEOUT_SECS 的读空闲上限：
//     翻译上游正常时首个 token 通常 <3s、相邻 token 几十 ms，15s 无数据基本等于假死。
//
// 并发收口 + 重试（2026-09-27 补）：
//   · 网关单用户并发任务上限 max_concurrent_jobs=2。实时录音时流式 ASR 会话占 1 个，
//     而客户端翻译有两个互相独立的 worker（草稿 + 定稿）会并发打网关 chat → 撞穿上限
//     → 429 too_many_jobs（2026-09-26 晚 20 分钟录音实测 27 次 429，且失败后不重试，
//     这些段落的定稿译文永久丢失）。GATEWAY_CHAT_SEMAPHORE（1 个许可）从根上保证
//     客户端任意时刻最多 1 个在飞的网关 chat 翻译，配合 ASR 的 1 个名额正好不超上限。
//   · 定稿翻译（max_retries > 0）对可重试错误（429 / 5xx / 网络错误）做有界指数退避；
//     草稿与翻译页单发调用传 0 不重试（草稿很快会被新一代取代，重试是白花钱）。

use futures_util::StreamExt;
use std::sync::LazyLock;

/// 流式读空闲上限（秒）：相邻两块 SSE 数据之间超过它即判假死，放弃本次翻译。
/// 为什么是 15s：与客户端流式 ASR 看门狗同一口径（网关 5s 心跳 × 3），
/// 且远小于整流 120s 总超时 —— 假死能在十几秒内暴露，而不是拖两分钟。
const IDLE_TIMEOUT_SECS: u64 = 15;

/// 网关 chat 翻译的进程级并发收口：1 个许可，草稿 worker 与定稿 worker 共用。
/// 为什么用信号量而不是「合并成一个 worker」：草稿（latest-wins）与定稿（FIFO）
/// 的调度语义本质不同，合并会把草稿排队在定稿后面失去实时性；信号量只约束
/// 「在飞请求数」，不动两条管线的调度。
static GATEWAY_CHAT_SEMAPHORE: LazyLock<tokio::sync::Semaphore> =
    LazyLock::new(|| tokio::sync::Semaphore::new(1));

/// 定稿翻译失败重试的退避档位（毫秒）：1s、2s、4s，最多重试 3 次。
/// 次数刻意压小：每次重试都占着信号量（别的翻译在排队），且仍失败的段落
/// 还有停止录音时的补译兜底（见 recording_commands.rs 停止流程 B2）。
const RETRY_BACKOFFS_MS: [u64; 3] = [1_000, 2_000, 4_000];

/// 该错误是否值得重试（纯函数，便于单测）。
/// 可重试 = 网关/链路的**临时性**故障：429（并发上限）、5xx（网关/上游异常）、
/// reqwest 网络错误（连接失败/流中断）、读空闲超时（链路假死，换新连接有机会恢复——
/// 与 ASR 看门狗「重连治假死」同一经验）。
/// 不可重试 = 重发必然再失败：配置缺失、不支持的方向、4xx 鉴权/积分类错误。
pub(crate) fn is_retryable_error(err: &str) -> bool {
    err.contains("too_many_jobs")
        || err.contains("(HTTP 429")
        || err.contains("(HTTP 5") // 500~599
        || err.contains("远程翻译请求失败") // reqwest 发送阶段网络错误
        || err.contains("远程翻译流错误") // 流式读中断
        || err.contains("远程翻译超时") // 读空闲看门狗判假死
}

/// translate_remote 的返回值：译文 + 「空结果」的细分原因。
/// 两种「空」病因完全不同（2026-09-28 生产实测，seq=44 三次尝试全空）：
///   ① 上游根本没产出（raw 为空）——多为推理预算/上游异常，网关侧免单；
///   ② 上游有产出但全是原文回声/非目标语言（raw 非空、清洗后为空）——
///      postprocess 按设计把纯拉丁行当回声丢弃，用户看到的同样是「没译文」，
///      但那次调用是**正常扣费**的。告警文案需要区分这两者。
pub struct RemoteTranslation {
    /// 清洗后的译文（与旧版 translate_remote 的 Ok 值一致）。
    pub text: String,
    /// text 为空时的细分：true = 上述 ②（有产出被清洗清空）；false = ①（上游真空）
    /// 或 text 非空（无意义）。
    pub emptied_by_cleanup: bool,
}

pub async fn translate_remote(
    text: &str,
    direction: &str,
    asr_mode: bool,
    context: Option<&str>,
    mut on_token: Option<&mut (dyn FnMut(&str) + Send)>,
    max_retries: u32,
) -> Result<RemoteTranslation, String> {
    let Some((src, tgt)) = super::llm::parse_direction(direction) else {
        return Err(format!("不支持的翻译方向: {}", direction));
    };
    let api_base = crate::audio::transcription::remote_api_base()
        .ok_or_else(|| "远程服务未配置（缺少服务器地址）".to_string())?;
    let license = crate::audio::transcription::get_remote_license();
    if license.is_empty() {
        return Err("远程服务未配置（缺少授权码）".to_string());
    }

    let prompt = super::llm::build_prompt(text, src, tgt, asr_mode, context);
    let translate_model = crate::audio::transcription::get_remote_translate_model();
    let model_name = if translate_model.is_empty() {
        "remote".to_string()
    } else {
        translate_model
    };
    let body = serde_json::json!({
        "model": model_name,
        // 标明用途：网关据此在积分流水里写「翻译 · 模型名」（不上行到上游）
        "feature": "translate",
        "messages": [{ "role": "user", "content": prompt }],
        // 豆包机器翻译等专用 API 需要原文与语言代码（其它 LLM 上游会忽略这些字段，
        // 网关在转发前会删除）。src/tgt 是客户端语言代码，与豆包 ISO 639-1 一致。
        "source_text": text,
        "source_lang": src,
        "target_lang": tgt,
        "stream": true,
        "temperature": 0.3,
        "max_tokens": (text.chars().count() * 2).clamp(64, 1024) as u32,
    });

    // 并发收口（见文件头注释）：草稿/定稿两条路径都在此拿许可，
    // 客户端任意时刻最多 1 个在飞的网关 chat 翻译。
    // 许可在整个重试期间持有（含退避）：退避时不放行别的请求，
    // 否则退避窗口里照样可能撞并发上限。
    let _permit = GATEWAY_CHAT_SEMAPHORE
        .acquire()
        .await
        .map_err(|e| format!("翻译并发信号量异常: {}", e))?;

    // 任务会话（2026-09-22）：让网关把本次录音的几百次逐块翻译聚成
    // 「一次录音 · 翻译」一个统计项，而不是几百行流水。见 src/task_session.rs。
    let session = crate::task_session::current_session();
    // 请求 id（幂等键）：一次**逻辑请求**只生成一次，重试必须复用同一个 id ——
    // 网关按 (用户, request id) 去重，重发直接返回首次结果、不重复扣费。
    // 所以重试循环在这个函数内部、id 生成**之后**。
    let request_id = crate::task_session::new_request_id();

    let client = reqwest::Client::new();
    let mut attempt = 0u32;
    loop {
        match translate_remote_once(
            &client,
            &api_base,
            &license,
            &session,
            &request_id,
            &body,
            on_token.as_mut().map(|c| &mut **c),
        )
        .await
        {
            Ok(raw) => {
                let cleaned = super::llm::postprocess(&raw, src, tgt);
                if !cleaned.trim().is_empty() {
                    return Ok(RemoteTranslation {
                        text: cleaned,
                        emptied_by_cleanup: false,
                    });
                }
                // 清洗后为空（2026-09-28 生产实测：qwen-flash 对句中截断的长英文句
                // 返回英文续写/回声而非译文，postprocess 按设计把纯拉丁行滤空）。
                // 用户什么都没拿到 → 与可重试错误同等对待：退避后换上游「心情」再试；
                // 重试复用同一幂等键。真空（raw 为空）那次网关免单，回声那次已扣费，
                // 两种都值得用剩余重试预算换一次拿到译文的机会。
                if attempt < max_retries {
                    let backoff =
                        RETRY_BACKOFFS_MS[(attempt as usize).min(RETRY_BACKOFFS_MS.len() - 1)];
                    attempt += 1;
                    log::warn!(
                        "翻译清洗后为空（{}），{}ms 后第 {}/{} 次重试",
                        if raw.trim().is_empty() {
                            "上游未产出"
                        } else {
                            "上游输出为回声/非目标语言"
                        },
                        backoff,
                        attempt,
                        max_retries,
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
                    continue;
                }
                return Ok(RemoteTranslation {
                    text: String::new(),
                    emptied_by_cleanup: !raw.trim().is_empty(),
                });
            }
            Err(e) => {
                if attempt < max_retries && is_retryable_error(&e) {
                    let backoff =
                        RETRY_BACKOFFS_MS[(attempt as usize).min(RETRY_BACKOFFS_MS.len() - 1)];
                    attempt += 1;
                    log::warn!(
                        "远程翻译失败（可重试），{}ms 后第 {}/{} 次重试: {}",
                        backoff,
                        attempt,
                        max_retries,
                        e
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

/// 单次网关 chat 翻译请求（不重试）。重试策略与幂等键复用见 `translate_remote`。
///
/// ⚠️ 中途失败（流中断/假死）后的重试会让调用方的 token 回调从头发送增量：
/// 调用方若自己累积 partial（如定稿 worker），重试期间发出的 is_partial 增量
/// 会短暂叠加重复文本，但定稿（is_partial=false）来自本函数返回值，不受影响。
/// 429/5xx 都在流开始**之前**返回，不会触发这种重复。
#[allow(clippy::too_many_arguments)]
async fn translate_remote_once<F: FnMut(&str) + Send + ?Sized>(
    client: &reqwest::Client,
    api_base: &str,
    license: &str,
    session: &str,
    request_id: &str,
    body: &serde_json::Value,
    mut on_token: Option<&mut F>,
) -> Result<String, String> {
    let resp = client
        .post(format!("{}/chat/completions", api_base))
        .bearer_auth(license)
        .header("x-vox-session", session)
        // 请求 id：逐块翻译量大，重发造成的小额重复扣费累积起来同样可观
        .header("x-vox-request-id", request_id)
        .json(body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("远程翻译请求失败: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!("远程翻译错误 (HTTP {}): {}", status, detail));
    }

    let mut raw = String::new();
    let mut buf = String::new();
    let mut stream = resp.bytes_stream();
    // 'sse：收到 [DONE] 直接结束（不再等服务端关连接——万一服务端 keep-alive 不收尾，
    // 旧写法会把已经拿全的译文拖到 120s 总超时才报错）
    'sse: loop {
        // 读空闲看门狗：任何一块数据都算存活；窗口内没有数据 = 假死，立刻放弃
        let next = tokio::time::timeout(
            std::time::Duration::from_secs(IDLE_TIMEOUT_SECS),
            stream.next(),
        )
        .await
        .map_err(|_| format!("远程翻译超时（{}s 无数据，疑似链路假死）", IDLE_TIMEOUT_SECS))?;
        let Some(chunk) = next else { break };
        let chunk = chunk.map_err(|e| format!("远程翻译流错误: {}", e))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf.drain(..=pos);
            let data = match line.strip_prefix("data:") {
                Some(d) => d.trim(),
                None => continue,
            };
            if data.is_empty() {
                continue;
            }
            if data == "[DONE]" {
                break 'sse;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                    raw.push_str(delta);
                    if let Some(ref mut cb) = on_token {
                        cb(delta);
                    }
                }
            }
        }
    }

    Ok(raw)
}

#[cfg(test)]
mod tests {
    use super::is_retryable_error;

    #[test]
    fn retryable_429_and_too_many_jobs() {
        // 2026-09-26 晚生产事故的主错误形态：并发上限撞穿
        assert!(is_retryable_error(
            "远程翻译错误 (HTTP 429 Too Many Requests): {\"error\":\"too_many_jobs\"}"
        ));
        assert!(is_retryable_error("too_many_jobs"));
    }

    #[test]
    fn retryable_5xx() {
        assert!(is_retryable_error("远程翻译错误 (HTTP 500 Internal Server Error): x"));
        assert!(is_retryable_error("远程翻译错误 (HTTP 502 Bad Gateway): x"));
        assert!(is_retryable_error("远程翻译错误 (HTTP 599): x"));
    }

    #[test]
    fn retryable_network_and_stall() {
        // reqwest 发送失败 / 流中断 / 读空闲假死：换新连接有机会恢复
        assert!(is_retryable_error("远程翻译请求失败: error sending request"));
        assert!(is_retryable_error("远程翻译流错误: connection reset"));
        assert!(is_retryable_error("远程翻译超时（15s 无数据，疑似链路假死）"));
    }

    #[test]
    fn not_retryable_config_and_client_errors() {
        // 配置缺失与 4xx（鉴权/积分/参数）：重发必然再失败
        assert!(!is_retryable_error("远程服务未配置（缺少授权码）"));
        assert!(!is_retryable_error("不支持的翻译方向: zh-xx"));
        assert!(!is_retryable_error("远程翻译错误 (HTTP 401 Unauthorized): 授权码无效"));
        assert!(!is_retryable_error("远程翻译错误 (HTTP 402 Payment Required): 积分不足"));
        assert!(!is_retryable_error("远程翻译错误 (HTTP 400 Bad Request): x"));
    }
}
