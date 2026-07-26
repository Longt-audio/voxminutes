// translation/llm.rs
//
// Hy-MT2 GGUF 翻译引擎（13 种常用语言互译），通过共享 llama-helper sidecar
// （crate::llama_sidecar）推理。Prompt 模板、采样参数与输出清洗移植自
// 参考实现 reference_code/backend/app/translate_engine.py。

use crate::llama_sidecar::{self, GenerateParams};

/// 模型目录（<models>/ 下），对应 model_download 注册表条目 hy-mt2-1.8b-q4_k_m。
const MODEL_DIR: &str = "hy-mt2-1.8b";

// ── 聊天模板（Hy-MT2 特殊 token）──────────────────────────────────────────────
//
// 注意：必须使用全角竖线 ｜ (U+FF5C) 和下划线 ▁ (U+2581)，与模型 GGUF
// tokenizer 的实际 token 文本一致；ASCII 竖线 | 不会被识别为特殊 token。

/// BOS + User 起始标记
const CHAT_PREFIX: &str =
    "<\u{FF5C}hy_begin\u{2581}of\u{2581}sentence\u{FF5C}><\u{FF5C}hy_User\u{FF5C}>";
/// Assistant 起始标记（触发模型生成）
const CHAT_SUFFIX: &str = "<\u{FF5C}hy_Assistant\u{FF5C}>";

/// 停止序列：模型切换角色或句子结束即停。
const STOP_TOKENS: &[&str] = &[
    "<\u{FF5C}hy_User\u{FF5C}>",
    "<\u{FF5C}hy_Assistant\u{FF5C}>",
    "<\u{FF5C}hy_end\u{2581}of\u{2581}sentence\u{FF5C}>",
    "<|endoftext|>",
];

// ── 采样参数（移植自参考实现 TranslationEngine 默认值）────────────────────────

const CONTEXT_SIZE: u32 = 4096;
const TEMPERATURE: f32 = 0.3;
const TOP_K: i32 = 20;
const TOP_P: f32 = 0.6;
const REPEAT_PENALTY: f32 = 1.15;
const FREQUENCY_PENALTY: f32 = 0.05;
// 韩语输出重复率偏高，参考实现 translate_engine.py 对韩语目标加大惩罚
const KO_REPEAT_PENALTY: f32 = 1.30;
const KO_FREQUENCY_PENALTY: f32 = 0.15;

/// 语言表：code → (中文名, 英文名)。
const LANG_TABLE: &[(&str, &str, &str)] = &[
    ("zh", "中文", "Chinese"),
    ("en", "英语", "English"),
    ("ja", "日语", "Japanese"),
    ("ko", "韩语", "Korean"),
    ("fr", "法语", "French"),
    ("de", "德语", "German"),
    ("es", "西班牙语", "Spanish"),
    ("ru", "俄语", "Russian"),
    ("pt", "葡萄牙语", "Portuguese"),
    ("zh-Hant", "繁体中文", "Traditional Chinese"),
    ("yue", "粤语", "Cantonese"),
    ("th", "泰语", "Thai"),
    ("vi", "越南语", "Vietnamese"),
];

/// 支持的目标语言代码（"auto" 之外），供 mod.rs/commands.rs 校验用。
pub(crate) const SUPPORTED_TARGET_LANGS: &[&str] = &[
    "zh", "en", "ja", "ko", "fr", "de", "es", "ru", "pt", "zh-Hant", "yue", "th", "vi",
];

/// 语言名表：(中文名, 英文名)；未知 code 返回空串。
fn lang_names(code: &str) -> (&'static str, &'static str) {
    LANG_TABLE
        .iter()
        .find(|(c, _, _)| *c == code)
        .map(|(_, zh, en)| (*zh, *en))
        .unwrap_or(("", ""))
}

/// 中文系语言（指令语言与回声行过滤按此判断）。
pub(crate) fn is_chinese_family(code: &str) -> bool {
    matches!(code, "zh" | "zh-Hant" | "yue")
}

/// 解析 "{src}-{tgt}" 方向；src/tgt 都必须在语言表内。
/// 注意 code 自身可能含连字符（如 zh-Hant），故按已知代码表匹配而非简单 split('-')。
pub(crate) fn parse_direction(direction: &str) -> Option<(&str, &str)> {
    for (code, _, _) in LANG_TABLE {
        if let Some(rest) = direction.strip_prefix(code) {
            if let Some(tgt) = rest.strip_prefix('-') {
                if SUPPORTED_TARGET_LANGS.contains(&tgt) {
                    return Some((code, tgt));
                }
            }
        }
    }
    None
}

/// 构建 Hy-MT2 聊天模板 prompt。
/// 普通模式：单行指令 + 原文；asr_mode：针对语音转录文本的纠错翻译指令
/// （完整 6 条要求，移植自参考实现的 build_asr_translate_prompt）。
/// src 或 tgt 属于中文系（zh/zh-Hant/yue）时用中文指令，否则用英文指令。
pub(crate) fn build_prompt(text: &str, source_lang: &str, target_lang: &str, asr_mode: bool) -> String {
    let user_text = if asr_mode {
        let (_, src_en) = lang_names(source_lang);
        let (_, tgt_en) = lang_names(target_lang);
        let instruction = if is_chinese_family(source_lang) || is_chinese_family(target_lang) {
            // 涉及中文时统一使用中文指令（参考实现的 has_zh 分支）
            format!(
                "将以下{src_en}语音转录文本翻译为{tgt_en}。\n\
                 要求：\n\
                 1. 只输出{tgt_en}译文，严禁输出原文、双语对照、原文片段或重复原文；\n\
                 2. 直接开始翻译，不要写“翻译：”“{tgt_en}：”等任何前缀；\n\
                 3. 修正识别错误和同音词；\n\
                 4. 省略语气词；\n\
                 5. 输出流畅自然的口语翻译；\n\
                 6. 不要解释，不要备注。"
            )
        } else {
            format!(
                "Translate the following {src_en} spoken transcript into {tgt_en}.\n\
                 Requirements:\n\
                 1. Output ONLY the {tgt_en} translation. \
                 Do NOT output the original text, bilingual pairs, source fragments, or repeated source.\n\
                 2. Start directly with the translation; \
                 do not write prefixes like \"Translation:\" or \"{tgt_en}:\".\n\
                 3. Fix ASR errors and homophones.\n\
                 4. Omit filler words.\n\
                 5. Produce a fluent, natural, conversational translation.\n\
                 6. Do not explain or add notes."
            )
        };
        format!("{instruction}\n\nSource: {text}\n\nTarget ({tgt_en}):")
    } else {
        let (tgt_native, tgt_en) = lang_names(target_lang);
        let instruction = if is_chinese_family(source_lang) {
            format!(
                "将以下文本翻译为{tgt_native}，注意只需要输出翻译后的结果，不要额外解释"
            )
        } else {
            format!(
                "Translate the following text into {tgt_en}. Only output the translated result, without any additional explanation."
            )
        };
        format!("{instruction}:\n\n{text}")
    };
    format!("{CHAT_PREFIX}{user_text}{CHAT_SUFFIX}")
}

// ── 输出清洗（参考实现 _clean_*_translation_output 的 zh/en 精简版）────────────

/// 模型可能回声的常见前缀（仅保留 zh/en 相关项）。
const ECHO_PREFIXES: &[&str] = &[
    "Translation:", "translation:", "Translated:", "translated:",
    "Translate:", "translate:", "English:", "Chinese:",
    "译文：", "翻译：", "英文：", "中文：",
];

/// 剔除 `<｜hy_...｜>` / `<|hy_...|>` / `<│hy_...│>` 形式的特殊 token。
fn strip_special_tokens(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find('<') {
        out.push_str(&rest[..idx]);
        let tail = &rest[idx..];
        if is_hy_special_token(tail) {
            // 跳过整个 token（含结尾 '>'）
            let end = tail.find('>').map(|e| e + 1).unwrap_or(tail.len());
            rest = &tail[end..];
        } else {
            out.push('<');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

/// s 以 '<' 开头，判断是否为 `<[｜│|]hy_...[｜│|]>` 形式的特殊 token。
fn is_hy_special_token(s: &str) -> bool {
    let Some(after_lt) = s.strip_prefix('<') else { return false };
    let Some(delim) = after_lt.chars().next() else { return false };
    if !matches!(delim, '\u{FF5C}' | '\u{2502}' | '|') {
        return false;
    }
    let Some(body) = after_lt[delim.len_utf8()..].strip_prefix("hy_") else {
        return false;
    };
    let Some(end) = body.find('>') else { return false };
    let inner = &body[..end];
    matches!(inner.chars().last(), Some('\u{FF5C}' | '\u{2502}' | '|'))
}

/// 整行均为 CJK 字符（参考实现 _CJK_RE fullmatch：含空格的行不算）。
fn is_all_cjk(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| {
            matches!(c as u32,
                0x4E00..=0x9FFF | 0x3000..=0x303F | 0x3040..=0x309F | 0x30A0..=0x30FF | 0xAC00..=0xD7AF)
        })
}

fn is_latin_letter(c: char) -> bool {
    matches!(c as u32, 0x41..=0x5A | 0x61..=0x7A | 0x00C0..=0x024F | 0x1E00..=0x1EFF)
}

/// 非空白字符中拉丁字母占比 ≥ 70%（参考实现 _is_mostly_latin）。
fn is_mostly_latin(s: &str) -> bool {
    let non_space: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    if non_space.is_empty() {
        return false;
    }
    let latin = non_space.iter().filter(|c| is_latin_letter(**c)).count();
    latin * 10 >= non_space.len() * 7
}

/// 去重用的归一化：只保留字母数字并转小写，忽略标点/空白差异。
fn normalize_for_dedup(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// 折叠连续重复段落（模型陷入循环时的安全网；间隔不同内容的有意重复会保留）。
fn dedup_consecutive_paragraphs(text: &str) -> String {
    if text.trim().is_empty() {
        return text.to_string();
    }
    let paras: Vec<&str> = text.trim().split("\n\n").collect();
    if paras.len() <= 1 {
        return text.trim().to_string();
    }
    let mut kept: Vec<&str> = vec![paras[0]];
    let mut prev_norm = normalize_for_dedup(paras[0]);
    for para in &paras[1..] {
        let norm = normalize_for_dedup(para);
        if !norm.is_empty() && norm == prev_norm {
            continue;
        }
        kept.push(para);
        if !norm.is_empty() {
            prev_norm = norm;
        }
    }
    kept.join("\n\n")
}

/// 清洗模型输出：去特殊 token、去回声前缀、按源/目标脚本过滤回声行、
/// 折叠连续重复段落。
pub(crate) fn postprocess(text: &str, source_lang: &str, target_lang: &str) -> String {
    let mut out = strip_special_tokens(text).trim().to_string();

    // 1. 逐个剥离回声前缀（可能叠加多个）
    loop {
        let mut changed = false;
        for prefix in ECHO_PREFIXES {
            if out.starts_with(prefix) {
                out = out[prefix.len()..].trim_start().to_string();
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }

    // 2. 按脚本过滤原文回声行（其他语言对不做脚本过滤，避免误伤）
    let cjk_echo_source =
        is_chinese_family(source_lang) || source_lang == "ja" || source_lang == "ko";
    if target_lang == "en" && cjk_echo_source {
        // 目标为英文：丢弃整行 CJK 的行（大概率是原文回声）
        out = out
            .lines()
            .filter(|l| {
                let s = l.trim();
                !s.is_empty() && !is_all_cjk(s)
            })
            .collect::<Vec<_>>()
            .join("\n");
    } else if is_chinese_family(target_lang) {
        // 目标为中文系：丢弃纯拉丁回声行；遇到说明/列表标记即截断
        let mut filtered: Vec<&str> = Vec::new();
        let mut saw_target = false;
        for line in out.lines() {
            let stripped = line.trim();
            if stripped.is_empty() {
                filtered.push(line);
                continue;
            }
            if stripped == "---"
                || stripped == "***"
                || stripped.starts_with("**")
                || stripped.starts_with("* ")
                || stripped.starts_with("- ")
                || ["1. ", "2. ", "3. ", "4. ", "5. ", "6. "]
                    .iter()
                    .any(|p| stripped.starts_with(p))
            {
                break;
            }
            if stripped.contains("说明") || stripped.contains("Note") || stripped.contains("原文") {
                break;
            }
            if is_mostly_latin(stripped) {
                if saw_target {
                    // 已有译文后出现纯拉丁行：原文回声或备注，截断
                    break;
                }
                // 译文前的纯拉丁行：前缀回声，跳过
                continue;
            }
            saw_target = true;
            filtered.push(line);
        }
        out = filtered.join("\n").trim_end().to_string();
    }

    // 3. 折叠连续重复段落
    dedup_consecutive_paragraphs(&out).trim().to_string()
}

// ── 翻译入口 ──────────────────────────────────────────────────────────────────

/// 用 Hy-MT2 翻译一段文本。direction 为 "{src}-{tgt}" 形式（如 "zh-en"、
/// "en-zh-Hant"），src/tgt 须在语言表内。
/// asr_mode=true 使用语音转录纠错指令（实时翻译路径）。
/// on_token 提供时走 sidecar 流式协议，增量文本（未清洗的原始输出）逐个
/// 回调；返回值始终是清洗后的完整译文。
/// 阻塞调用，请放在 spawn_blocking 中执行。
pub fn translate(
    text: &str,
    direction: &str,
    asr_mode: bool,
    on_token: Option<&mut dyn FnMut(&str)>,
) -> Result<String, String> {
    let Some((source_lang, target_lang)) = parse_direction(direction) else {
        return Err(format!("不支持的翻译方向: {}", direction));
    };
    if source_lang == target_lang {
        return Err("源语言与目标语言相同，无需翻译".to_string());
    }
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    let model_path = llama_sidecar::find_gguf_model(MODEL_DIR)
        .ok_or_else(|| "Hy-MT2 翻译模型未安装，请先到设置页下载。".to_string())?;
    let helper_exe = llama_sidecar::resolve_helper_exe()
        .ok_or_else(|| "本地推理引擎（llama-helper）未找到，请重新安装应用".to_string())?;

    let prompt = build_prompt(text, source_lang, target_lang, asr_mode);
    // 输出预算按输入字符数估算（译文 token 数通常不超过原文字符数的两倍）
    let max_tokens = (text.chars().count() * 2).clamp(64, 1024) as u32;
    // 韩语目标重复率偏高，按参考实现加大惩罚
    let (repeat_penalty, frequency_penalty) = if target_lang == "ko" {
        (KO_REPEAT_PENALTY, KO_FREQUENCY_PENALTY)
    } else {
        (REPEAT_PENALTY, FREQUENCY_PENALTY)
    };

    let raw = llama_sidecar::blocking_generate(
        &helper_exe,
        GenerateParams {
            model_path: model_path.to_string_lossy().to_string(),
            prompt,
            max_tokens,
            context_size: CONTEXT_SIZE,
            temperature: TEMPERATURE,
            top_k: TOP_K,
            top_p: TOP_P,
            repeat_penalty: Some(repeat_penalty),
            frequency_penalty: Some(frequency_penalty),
            stop_tokens: STOP_TOKENS.iter().map(|s| s.to_string()).collect(),
            stream: on_token.is_some(),
        },
        on_token,
    )?;

    Ok(postprocess(&raw, source_lang, target_lang))
}

/// 启动预加载暖机：发一次最小 generate，使 llama-helper sidecar 启动并驻留
/// Hy-MT2 模型，消除首次翻译的冷启动等待。阻塞调用，请放在 spawn_blocking 中。
pub fn warmup() -> Result<(), String> {
    let model_path = llama_sidecar::find_gguf_model(MODEL_DIR)
        .ok_or_else(|| "Hy-MT2 翻译模型未安装，请先到设置页下载。".to_string())?;
    let helper_exe = llama_sidecar::resolve_helper_exe()
        .ok_or_else(|| "本地推理引擎（llama-helper）未找到，请重新安装应用".to_string())?;

    llama_sidecar::blocking_generate(
        &helper_exe,
        GenerateParams {
            model_path: model_path.to_string_lossy().to_string(),
            prompt: "Hi".to_string(),
            max_tokens: 1,
            context_size: CONTEXT_SIZE,
            temperature: TEMPERATURE,
            top_k: TOP_K,
            top_p: TOP_P,
            repeat_penalty: Some(REPEAT_PENALTY),
            frequency_penalty: Some(FREQUENCY_PENALTY),
            stop_tokens: Vec::new(),
            stream: false,
        },
        None,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_normal_zh_en() {
        let p = build_prompt("你好，世界", "zh", "en", false);
        assert!(p.starts_with(CHAT_PREFIX));
        assert!(p.ends_with(CHAT_SUFFIX));
        assert!(p.contains("将以下文本翻译为英语"));
        assert!(p.contains(":\n\n你好，世界"));
    }

    #[test]
    fn build_prompt_normal_en_zh() {
        let p = build_prompt("hello world", "en", "zh", false);
        assert!(p.starts_with(CHAT_PREFIX));
        assert!(p.ends_with(CHAT_SUFFIX));
        assert!(p.contains("Translate the following text into Chinese."));
        assert!(p.contains("hello world"));
    }

    #[test]
    fn build_prompt_asr_contains_instruction_and_source_target() {
        let p = build_prompt("今天天气不错", "zh", "en", true);
        assert!(p.starts_with(CHAT_PREFIX));
        assert!(p.ends_with(CHAT_SUFFIX));
        assert!(p.contains("将以下Chinese语音转录文本翻译为English。"));
        // 完整 6 条要求（中文版）
        assert!(p.contains("1. 只输出English译文，严禁输出原文、双语对照、原文片段或重复原文；"));
        assert!(p.contains("3. 修正识别错误和同音词；"));
        assert!(p.contains("6. 不要解释，不要备注。"));
        assert!(p.contains("Source: 今天天气不错"));
        assert!(p.contains("Target (English):"));
    }

    #[test]
    fn build_prompt_chat_tokens_use_fullwidth_pipe() {
        let p = build_prompt("x", "zh", "en", false);
        assert!(p.contains("<\u{FF5C}hy_begin\u{2581}of\u{2581}sentence\u{FF5C}>"));
        assert!(p.contains("<\u{FF5C}hy_User\u{FF5C}>"));
        assert!(p.contains("<\u{FF5C}hy_Assistant\u{FF5C}>"));
    }

    #[test]
    fn postprocess_strips_special_tokens_and_echo_prefix() {
        let out = postprocess(
            "<\u{FF5C}hy_Assistant\u{FF5C}>Translation: 你好<\u{FF5C}hy_end\u{2581}of\u{2581}sentence\u{FF5C}>",
            "en",
            "zh",
        );
        assert_eq!(out, "你好");
    }

    #[test]
    fn postprocess_strips_chinese_echo_prefix() {
        let out = postprocess("译文：今天天气很好", "zh", "en");
        // 前缀剥离后整行 CJK，对英文目标属于回声行，被过滤为空
        assert_eq!(out, "");
        let out2 = postprocess("翻译：The weather is nice today", "zh", "en");
        assert_eq!(out2, "The weather is nice today");
    }

    #[test]
    fn postprocess_drops_cjk_echo_lines_for_en_target() {
        let out = postprocess("This is fine.\n这是回声行", "zh", "en");
        assert_eq!(out, "This is fine.");
    }

    #[test]
    fn postprocess_drops_latin_echo_for_zh_target() {
        let out = postprocess("hello world echo line\n这是译文", "en", "zh");
        assert_eq!(out, "这是译文");
    }

    #[test]
    fn postprocess_collapses_repeated_paragraphs() {
        let out = postprocess("这是译文。\n\n这是译文。", "en", "zh");
        assert_eq!(out, "这是译文。");
    }

    #[test]
    fn parse_direction_common_pairs() {
        assert_eq!(parse_direction("zh-en"), Some(("zh", "en")));
        assert_eq!(parse_direction("en-ja"), Some(("en", "ja")));
        // code 自身含连字符的情况
        assert_eq!(parse_direction("zh-Hant-en"), Some(("zh-Hant", "en")));
        assert_eq!(parse_direction("en-zh-Hant"), Some(("en", "zh-Hant")));
        assert_eq!(parse_direction("yue-zh"), Some(("yue", "zh")));
        // 未知 code / 缺少 tgt 均拒绝
        assert_eq!(parse_direction("zh"), None);
        assert_eq!(parse_direction("zh-xx"), None);
        assert_eq!(parse_direction("xx-en"), None);
    }

    #[test]
    fn build_prompt_normal_zh_ja() {
        let p = build_prompt("你好，世界", "zh", "ja", false);
        assert!(p.starts_with(CHAT_PREFIX));
        assert!(p.ends_with(CHAT_SUFFIX));
        assert!(p.contains("将以下文本翻译为日语"));
    }

    #[test]
    fn build_prompt_normal_en_fr() {
        let p = build_prompt("hello world", "en", "fr", false);
        assert!(p.contains("Translate the following text into French."));
        assert!(p.contains("hello world"));
    }

    #[test]
    fn build_prompt_normal_zh_hant_target_uses_chinese_instruction() {
        let p = build_prompt("bonjour", "fr", "zh-Hant", false);
        assert!(p.contains("Translate the following text into Traditional Chinese."));
    }

    #[test]
    fn build_prompt_asr_non_chinese_pair_uses_english_instruction() {
        let p = build_prompt("hello world", "en", "ja", true);
        assert!(p.contains("Translate the following English spoken transcript into Japanese."));
        // 完整 6 条要求（英文版）
        assert!(p.contains("1. Output ONLY the Japanese translation."));
        assert!(p.contains("3. Fix ASR errors and homophones."));
        assert!(p.contains("6. Do not explain or add notes."));
        assert!(p.contains("Source: hello world"));
        assert!(p.contains("Target (Japanese):"));
    }

    #[test]
    fn build_prompt_asr_chinese_family_target_uses_chinese_instruction() {
        let p = build_prompt("bonjour le monde", "fr", "zh-Hant", true);
        assert!(p.contains("将以下French语音转录文本翻译为Traditional Chinese"));
        assert!(p.contains("Target (Traditional Chinese):"));
    }

    #[test]
    fn postprocess_keeps_cjk_lines_for_non_en_target() {
        // zh → ja：不做脚本过滤，CJK 行不应被误删
        let out = postprocess("こんにちは\n这是中文行", "zh", "ja");
        assert_eq!(out, "こんにちは\n这是中文行");
    }

    #[test]
    fn postprocess_drops_cjk_echo_lines_ja_to_en() {
        let out = postprocess("This is fine.\nこれは回声です", "ja", "en");
        assert_eq!(out, "This is fine.");
    }

    #[test]
    fn postprocess_drops_latin_echo_for_zh_hant_target() {
        let out = postprocess("echo line in latin\n這是譯文", "fr", "zh-Hant");
        assert_eq!(out, "這是譯文");
    }

    #[test]
    fn postprocess_no_script_filter_for_latin_pair() {
        // fr → de：不做脚本过滤，拉丁行全部保留
        let out = postprocess("Bonjour le monde\nHallo Welt", "fr", "de");
        assert_eq!(out, "Bonjour le monde\nHallo Welt");
    }
}
