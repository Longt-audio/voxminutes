// translation/engine.rs
//
// Local OPUS-MT translation engine (Marian encoder-decoder via ONNX Runtime).
// Models: Xenova/opus-mt-zh-en & Xenova/opus-mt-en-zh, int8 quantized.
// Each direction needs: encoder_model_int8.onnx, decoder_model_merged_int8.onnx,
// tokenizer.json, generation_config.json (source.spm/target.spm are kept for
// completeness but tokenizer.json is what we actually load).

use anyhow::{anyhow, Result};
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;
use std::sync::Mutex;
use tokenizers::Tokenizer;

pub const ENCODER_FILE: &str = "encoder_model_int8.onnx";
pub const DECODER_FILE: &str = "decoder_model_merged_int8.onnx";
pub const TOKENIZER_FILE: &str = "tokenizer.json";
pub const GENERATION_CONFIG_FILE: &str = "generation_config.json";

/// Files that must exist for a translation model directory to be loadable.
pub const REQUIRED_FILES: &[&str] = &[ENCODER_FILE, DECODER_FILE, TOKENIZER_FILE];

/// Load a Xenova-style tokenizer.json. These files carry
/// `"normalizer": {"type": "Precompiled", "precompiled_charsmap": null}`,
/// which the Rust `tokenizers` crate cannot parse (it expects a string).
/// Since the charsmap is null anyway, dropping the normalizer is a no-op
/// semantically — the Metaspace pre-tokenizer still handles the ▁ marker.
fn load_tokenizer(path: &Path) -> Result<Tokenizer> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("读取 tokenizer.json 失败: {}", e))?;
    let mut value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| anyhow!("解析 tokenizer.json 失败: {}", e))?;
    if value
        .get("normalizer")
        .and_then(|n| n.get("precompiled_charsmap"))
        .map(|c| c.is_null())
        .unwrap_or(false)
    {
        value["normalizer"] = serde_json::Value::Null;
    }
    let patched = serde_json::to_string(&value).map_err(|e| anyhow!("重写 tokenizer.json 失败: {}", e))?;
    Tokenizer::from_bytes(patched.as_bytes()).map_err(|e| anyhow!("加载翻译 tokenizer 失败: {}", e))
}

/// 翻译方向（由模型目录名决定，用于丢句检测的阈值选择）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    ZhEn,
    EnZh,
}

pub struct OpusMtEngine {
    // Session::run needs &mut self; guard with a mutex so the engine can be
    // shared as Arc across async tasks.
    encoder: Mutex<Session>,
    decoder: Mutex<Session>,
    tokenizer: Tokenizer,
    direction: Direction,
    decoder_start_token_id: i64,
    eos_token_id: i64,
    max_new_tokens: usize,
}

impl OpusMtEngine {
    pub fn load(dir: &Path) -> Result<Self> {
        let direction = match dir.file_name().and_then(|n| n.to_str()) {
            Some("opus-mt-zh-en") => Direction::ZhEn,
            _ => Direction::EnZh,
        };
        let encoder_path = dir.join(ENCODER_FILE);
        let decoder_path = dir.join(DECODER_FILE);
        let tokenizer_path = dir.join(TOKENIZER_FILE);
        for p in [&encoder_path, &decoder_path, &tokenizer_path] {
            if !p.exists() {
                return Err(anyhow!(
                    "翻译模型文件缺失: {}。请先在设置页下载翻译模型。",
                    p.display()
                ));
            }
        }

        // 翻译推理限制为 2 线程：ASR 解码也在同一 CPU 上跑（且多为 4 核低压本），
        // 翻译吃满核会拖慢实时转录。2 线程对句级翻译延迟影响很小。
        let encoder = Session::builder()
            .map_err(|e| anyhow!("初始化 ONNX Runtime 失败: {}", e))?
            .with_intra_threads(2)
            .map_err(|e| anyhow!("配置线程失败: {}", e))?
            .commit_from_file(&encoder_path)
            .map_err(|e| anyhow!("加载翻译 encoder 失败: {}", e))?;
        let decoder = Session::builder()
            .map_err(|e| anyhow!("初始化 ONNX Runtime 失败: {}", e))?
            .with_intra_threads(2)
            .map_err(|e| anyhow!("配置线程失败: {}", e))?
            .commit_from_file(&decoder_path)
            .map_err(|e| anyhow!("加载翻译 decoder 失败: {}", e))?;
        let tokenizer = load_tokenizer(&tokenizer_path)?;

        // Defaults match the Xenova OPUS-MT generation_config.json.
        let mut decoder_start: i64 = 65000;
        let mut eos: i64 = 0;
        let cfg_path = dir.join(GENERATION_CONFIG_FILE);
        if let Ok(text) = std::fs::read_to_string(&cfg_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                decoder_start = v["decoder_start_token_id"].as_i64().unwrap_or(decoder_start);
                eos = v["eos_token_id"].as_i64().unwrap_or(eos);
            }
        }

        // Log actual input/output names once so mismatches are easy to diagnose.
        let enc_inputs: Vec<String> = encoder.inputs.iter().map(|i| i.name.clone()).collect();
        let dec_inputs: Vec<String> = decoder.inputs.iter().map(|i| i.name.clone()).collect();
        let dec_outputs: Vec<String> = decoder.outputs.iter().map(|o| o.name.clone()).collect();
        log::info!(
            "OpusMtEngine loaded from {} (encoder inputs: {:?}, decoder inputs: {:?}, decoder outputs: {:?})",
            dir.display(),
            enc_inputs,
            dec_inputs,
            dec_outputs
        );

        Ok(Self {
            encoder: Mutex::new(encoder),
            decoder: Mutex::new(decoder),
            tokenizer,
            direction,
            decoder_start_token_id: decoder_start,
            eos_token_id: eos,
            max_new_tokens: 256,
        })
    }

    /// Translate `text` with beam search (beam=3 + 长度归一化)。
    /// 带丢句检测：长度比异常时按句界两段式回退翻译（方案 A）。
    /// Blocking — call via spawn_blocking.
    pub fn translate(&self, text: &str) -> Result<String> {
        let out = self.decode(text, DEFAULT_BEAM_SIZE)?;
        if !self.looks_truncated(text, &out) {
            return Ok(out);
        }
        log::warn!(
            "疑似丢句（输入 {} 字符 → 译文 {} 字符），启用两段式回退翻译",
            text.chars().filter(|c| !c.is_whitespace()).count(),
            out.chars().filter(|c| !c.is_whitespace()).count()
        );
        match self.translate_in_two_parts(text)? {
            Some(joined) => Ok(joined),
            None => Ok(out), // 无法切分时退回原结果
        }
    }

    /// Translate `text` with greedy decoding（beam=1，速度优先，用于流式快照）。
    pub fn translate_greedy(&self, text: &str) -> Result<String> {
        self.decode(text, 1)
    }

    /// 疑似丢句检测：译文/原文（非空白字符）长度比低于方向阈值。
    /// 阈值故意放宽（宁可误判多译一次，也不放过丢句）。
    fn looks_truncated(&self, input: &str, output: &str) -> bool {
        let in_chars = input.chars().filter(|c| !c.is_whitespace()).count();
        let out_chars = output.chars().filter(|c| !c.is_whitespace()).count();
        if in_chars < 40 || out_chars == 0 {
            return false;
        }
        let ratio = out_chars as f32 / in_chars as f32;
        match self.direction {
            // 中文比英文紧凑得多，完整 en→zh 约 0.3~0.7，丢句案例 ~0.17
            Direction::EnZh => ratio < 0.28,
            // 英译中反向：完整 zh→en 约 1.5~3.0
            Direction::ZhEn => ratio < 1.2,
        }
    }

    /// 在句界把输入切成两段分别翻译并拼接（仅在疑似丢句时触发）。
    fn translate_in_two_parts(&self, text: &str) -> Result<Option<String>> {
        let Some((head, tail)) = split_at_sentence_middle(text) else {
            return Ok(None);
        };
        let head_out = self.decode(head, DEFAULT_BEAM_SIZE)?;
        let tail_out = self.decode(tail, DEFAULT_BEAM_SIZE)?;
        if head_out.is_empty() || tail_out.is_empty() {
            return Ok(None);
        }
        Ok(Some(join_translation_parts(&head_out, &tail_out)))
    }

    /// 统一的编码器 + 解码器流程。beam_size=1 时退化为贪心解码。
    fn decode(&self, text: &str, beam_size: usize) -> Result<String> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow!("分词失败: {}", e))?;
        let ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&x| x as i64)
            .collect();
        if ids.is_empty() {
            return Ok(String::new());
        }
        let seq_len = ids.len() as i64;

        // ── Encoder ──
        let input_ids = Tensor::from_array((vec![1i64, seq_len], ids))
            .map_err(|e| anyhow!("创建 input_ids 张量失败: {}", e))?;
        let attention = Tensor::from_array((vec![1i64, seq_len], mask.clone()))
            .map_err(|e| anyhow!("创建 attention_mask 张量失败: {}", e))?;

        let mut encoder = self.encoder.lock().unwrap();
        let enc_outputs = encoder
            .run(ort::inputs![
                "input_ids" => input_ids,
                "attention_mask" => attention
            ])
            .map_err(|e| anyhow!("encoder 推理失败: {}", e))?;
        let (hs_shape, hs_data) = enc_outputs["last_hidden_state"]
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("读取 encoder 输出失败: {}", e))?;
        let hidden_shape: Vec<i64> = hs_shape.iter().copied().collect();
        let hidden_vec: Vec<f32> = hs_data.to_vec();
        drop(enc_outputs);
        drop(encoder);

        let mut decoder = self.decoder.lock().unwrap();
        let mut ctx = DecodeCtx {
            hidden_shape,
            hidden_vec,
            mask,
            seq_len,
        };
        let generated = if beam_size <= 1 {
            self.greedy_decode(&mut decoder, &mut ctx)?
        } else {
            self.beam_decode(&mut decoder, &mut ctx, beam_size)?
        };
        drop(decoder);

        let token_ids: Vec<u32> = generated.iter().map(|&x| x as u32).collect();
        let text_out = self
            .tokenizer
            .decode(&token_ids, true)
            .map_err(|e| anyhow!("解码失败: {}", e))?;
        Ok(text_out.trim().to_string())
    }

    /// 单步 decoder 推理（KV 缓存）。
    /// 返回 (最后一个位置的 logits, 新的 decoder past, 第 0 步时的 encoder past)。
    /// 模型行为（已实测）：每次调用必须提供全部 24 个 past_key_values.*；
    /// 第 0 步 use_cache_branch=false 返回真实 present.*（encoder 全长）；
    /// 后续步 use_cache_branch=true 时 present.encoder.* 是形状 [0,...] 的空占位，
    /// 因此 encoder KV 必须一直沿用第 0 步的输出。
    #[allow(clippy::too_many_arguments)]
    fn decoder_step(
        &self,
        decoder: &mut Session,
        ctx: &DecodeCtx,
        prev_token: i64,
        decoder_past: &[(Vec<i64>, Vec<f32>)],
        encoder_past: &[(Vec<i64>, Vec<f32>)],
        first_step: bool,
        has_enc_mask: bool,
        allocator: &ort::memory::Allocator,
    ) -> Result<StepOutput> {
        let dec_ids = Tensor::from_array((vec![1i64, 1i64], vec![prev_token]))
            .map_err(|e| anyhow!("创建 decoder 输入失败: {}", e))?;
        let enc_hs = Tensor::from_array((ctx.hidden_shape.clone(), ctx.hidden_vec.clone()))
            .map_err(|e| anyhow!("创建 encoder_hidden_states 失败: {}", e))?;
        let use_cache = Tensor::from_array((vec![1i64], vec![!first_step]))
            .map_err(|e| anyhow!("创建 use_cache_branch 失败: {}", e))?;

        let mut inputs: Vec<(std::borrow::Cow<'_, str>, ort::session::SessionInputValue<'_>)> = vec![
            ("input_ids".into(), dec_ids.into()),
            ("encoder_hidden_states".into(), enc_hs.into()),
            ("use_cache_branch".into(), use_cache.into()),
        ];
        let enc_mask_tensor;
        if has_enc_mask {
            enc_mask_tensor = Tensor::from_array((vec![1i64, ctx.seq_len], ctx.mask.clone()))
                .map_err(|e| anyhow!("创建 encoder_attention_mask 失败: {}", e))?;
            inputs.push(("encoder_attention_mask".into(), enc_mask_tensor.into()));
        }
        // past 张量：decoder 12 个 + encoder 12 个，按声明顺序交错（每层 dec.k, dec.v, enc.k, enc.v）
        for i in 0..6 {
            for (side_past, side_name) in [(decoder_past, "decoder"), (encoder_past, "encoder")] {
                for (idx, kv) in ["key", "value"].iter().enumerate() {
                    let (shape, data) = &side_past[i * 2 + idx];
                    let name = format!("past_key_values.{}.{}.{}", i, side_name, kv);
                    let t = if data.is_empty() {
                        Tensor::<f32>::new(allocator, shape.clone())
                            .map_err(|e| anyhow!("创建空 {} 失败: {}", name, e))?
                    } else {
                        Tensor::from_array((shape.clone(), data.clone()))
                            .map_err(|e| anyhow!("创建 {} 失败: {}", name, e))?
                    };
                    inputs.push((name.into(), t.into()));
                }
            }
        }

        let outs = decoder
            .run(inputs)
            .map_err(|e| anyhow!("decoder 推理失败: {}", e))?;

        let (logits_shape, logits) = outs["logits"]
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("读取 decoder 输出失败: {}", e))?;
        let dims: Vec<i64> = logits_shape.iter().copied().collect();
        if dims.len() != 3 || dims[1] == 0 {
            return Err(anyhow!("decoder 输出形状异常: {:?}", dims));
        }
        let vocab = dims[2] as usize;
        let offset = (dims[1] as usize - 1) * vocab;
        let last_logits: Vec<f32> = logits[offset..offset + vocab].to_vec();

        let mut new_decoder_past: Vec<(Vec<i64>, Vec<f32>)> = Vec::with_capacity(12);
        let mut new_encoder_past: Option<Vec<(Vec<i64>, Vec<f32>)>> = None;
        for i in 0..6 {
            for kv in ["key", "value"] {
                for (side, is_dec) in [("decoder", true), ("encoder", false)] {
                    let name = format!("present.{}.{}.{}", i, side, kv);
                    let should_read = is_dec || first_step;
                    if !should_read {
                        continue;
                    }
                    let (shape, data) = outs[name.as_str()]
                        .try_extract_tensor::<f32>()
                        .map_err(|e| anyhow!("读取 {} 失败: {}", name, e))?;
                    let entry = (shape.iter().copied().collect(), data.to_vec());
                    if is_dec {
                        new_decoder_past.push(entry);
                    } else {
                        new_encoder_past.get_or_insert_with(Vec::new).push(entry);
                    }
                }
            }
        }

        Ok(StepOutput {
            last_logits,
            decoder_past: new_decoder_past,
            encoder_past: new_encoder_past,
        })
    }

    /// 贪心解码（beam=1，速度快，用于流式快照）。
    fn greedy_decode(&self, decoder: &mut Session, ctx: &DecodeCtx) -> Result<Vec<i64>> {
        let empty12: Vec<(Vec<i64>, Vec<f32>)> =
            (0..12).map(|_| (vec![1, 8, 0, 64], Vec::new())).collect();
        let allocator = ort::memory::Allocator::default();
        let has_enc_mask = decoder.inputs.iter().any(|i| i.name == "encoder_attention_mask");

        let mut dec_past = empty12.clone();
        let mut enc_past = empty12;
        let mut prev = self.decoder_start_token_id;
        let mut out: Vec<i64> = Vec::new();

        for step in 0..self.max_new_tokens {
            let so = self.decoder_step(
                decoder, ctx, prev, &dec_past, &enc_past, step == 0, has_enc_mask, &allocator,
            )?;
            dec_past = so.decoder_past;
            if let Some(ep) = so.encoder_past {
                enc_past = ep;
            }
            let next = so
                .last_logits
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i as i64)
                .unwrap_or(self.eos_token_id);
            if next == self.eos_token_id {
                break;
            }
            out.push(next);
            prev = next;
        }
        Ok(out)
    }

    /// Beam search 解码（num_beams + 长度归一化打分）。
    /// 相比贪心能显著减少多句输入下的"欠翻译"（丢句）问题——
    /// 这也是模型 generation_config 推荐的解码方式（num_beams + renormalize_logits）。
    fn beam_decode(&self, decoder: &mut Session, ctx: &DecodeCtx, beam_size: usize) -> Result<Vec<i64>> {
        let empty12: Vec<(Vec<i64>, Vec<f32>)> =
            (0..12).map(|_| (vec![1, 8, 0, 64], Vec::new())).collect();
        let allocator = ort::memory::Allocator::default();
        let has_enc_mask = decoder.inputs.iter().any(|i| i.name == "encoder_attention_mask");

        let mut enc_past = empty12.clone();
        let mut enc_ready = false;

        struct BeamState {
            tokens: Vec<i64>,
            score: f32,
            decoder_past: std::rc::Rc<Vec<(Vec<i64>, Vec<f32>)>>,
        }

        let mut beams: Vec<BeamState> = vec![BeamState {
            tokens: Vec::new(),
            score: 0.0,
            decoder_past: std::rc::Rc::new(empty12),
        }];
        let mut finished: Vec<(Vec<i64>, f32)> = Vec::new(); // (tokens, raw_score)

        for _step in 0..self.max_new_tokens {
            if beams.is_empty() {
                break;
            }
            // (parent_shared_past, tokens, score)
            let mut expansions: Vec<(
                std::rc::Rc<Vec<(Vec<i64>, Vec<f32>)>>,
                Vec<i64>,
                f32,
            )> = Vec::new();

            for beam in &beams {
                let prev = beam
                    .tokens
                    .last()
                    .copied()
                    .unwrap_or(self.decoder_start_token_id);
                let so = self.decoder_step(
                    decoder,
                    ctx,
                    prev,
                    &beam.decoder_past,
                    &enc_past,
                    !enc_ready,
                    has_enc_mask,
                    &allocator,
                )?;
                if let Some(ep) = so.encoder_past {
                    enc_past = ep;
                    enc_ready = true;
                }
                let shared_past = std::rc::Rc::new(so.decoder_past);
                let lsm = log_softmax(&so.last_logits);
                for (tid, lp) in top_k(&lsm, beam_size * 2) {
                    if tid == self.eos_token_id {
                        finished.push((beam.tokens.clone(), beam.score + lp));
                    } else {
                        let mut toks = beam.tokens.clone();
                        toks.push(tid);
                        expansions.push((std::rc::Rc::clone(&shared_past), toks, beam.score + lp));
                    }
                }
            }

            if expansions.is_empty() {
                break;
            }
            expansions.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            expansions.truncate(beam_size);
            beams = expansions
                .into_iter()
                .map(|(p, t, s)| BeamState {
                    tokens: t,
                    score: s,
                    decoder_past: p,
                })
                .collect();

            if finished.len() >= beam_size {
                break;
            }
        }

        // 已完成 beam 按长度归一化打分排序
        if !finished.is_empty() {
            finished.sort_by(|a, b| {
                let na = a.1 / (a.0.len().max(1) as f32).powf(LENGTH_PENALTY);
                let nb = b.1 / (b.0.len().max(1) as f32).powf(LENGTH_PENALTY);
                nb.partial_cmp(&na).unwrap_or(std::cmp::Ordering::Equal)
            });
            return Ok(finished[0].0.clone());
        }
        // 没有完成的 beam：取原始分最高的未完成 beam
        beams.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(beams.into_iter().next().map(|b| b.tokens).unwrap_or_default())
    }
}

const DEFAULT_BEAM_SIZE: usize = 3;
/// 长度惩罚 > 1 鼓励更长的完整翻译，对抗 beam 偏向短输出的倾向（NMT 欠翻译对策）。
const LENGTH_PENALTY: f32 = 1.2;

struct DecodeCtx {
    hidden_shape: Vec<i64>,
    hidden_vec: Vec<f32>,
    mask: Vec<i64>,
    seq_len: i64,
}

struct StepOutput {
    last_logits: Vec<f32>,
    decoder_past: Vec<(Vec<i64>, Vec<f32>)>,
    encoder_past: Option<Vec<(Vec<i64>, Vec<f32>)>>,
}

fn log_softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let sum = logits.iter().map(|x| (x - max).exp()).sum::<f32>();
    let logsum = max + sum.ln();
    logits.iter().map(|x| x - logsum).collect()
}

fn top_k(lsm: &[f32], k: usize) -> Vec<(i64, f32)> {
    let k = k.min(lsm.len());
    if k == 0 {
        return Vec::new();
    }
    let mut idx: Vec<usize> = (0..lsm.len()).collect();
    idx.select_nth_unstable_by(k - 1, |&a, &b| {
        lsm[b].partial_cmp(&lsm[a]).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut top: Vec<usize> = idx[..k].to_vec();
    top.sort_unstable_by(|&a, &b| {
        lsm[b].partial_cmp(&lsm[a]).unwrap_or(std::cmp::Ordering::Equal)
    });
    top.into_iter().map(|i| (i as i64, lsm[i])).collect()
}

/// 在最接近文本中点的句子边界处切分为 (head, tail)。
/// 句界 = 。！？.!? 后接空白/引号/结尾的位置。找不到合适句界返回 None。
fn split_at_sentence_middle(text: &str) -> Option<(&str, &str)> {
    let char_indices: Vec<(usize, char)> = text.char_indices().collect();
    let total_chars = char_indices.len();
    if total_chars < 40 {
        return None;
    }

    let mut boundaries: Vec<usize> = Vec::new(); // byte offsets（句界之后）
    for (i, (byte_idx, c)) in char_indices.iter().enumerate() {
        if !matches!(c, '。' | '！' | '？' | '.' | '!' | '?') {
            continue;
        }
        let next = char_indices.get(i + 1).map(|(_, c)| *c);
        match next {
            None => boundaries.push(text.len()),
            Some(n) if n.is_whitespace() || matches!(n, '"' | '\u{201D}' | '\u{300D}' | '\u{FF09}' | '」' | '）') || (n as u32) >= 0x4E00 => {
                boundaries.push(byte_idx + c.len_utf8())
            }
            _ => {}
        }
    }

    // 选择最接近中点的句界
    let mid = total_chars / 2;
    let best = boundaries
        .iter()
        .filter(|&&b| b > 0 && b < text.len())
        .min_by_key(|&&b| {
            let chars_before = text[..b].chars().count() as isize;
            (chars_before - mid as isize).abs()
        })?;

    let head = text[..*best].trim();
    let tail = text[*best..].trim();
    if head.chars().count() < 10 || tail.chars().count() < 10 {
        return None;
    }
    Some((head, tail))
}

/// 拼接两段译文：相邻侧若任一端是 CJK 则直接相连，否则补一个空格。
fn join_translation_parts(a: &str, b: &str) -> String {
    let a_last = a.chars().last();
    let b_first = b.chars().next();
    let cjk = |c: char| {
        let u = c as u32;
        (0x4E00..=0x9FFF).contains(&u) || (0x3400..=0x4DBF).contains(&u)
    };
    match (a_last, b_first) {
        (Some(l), Some(f)) if cjk(l) && cjk(f) => format!("{}{}", a, b),
        _ => format!("{} {}", a, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_dir(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../models")
            .join(name)
    }

    #[test]
    fn smoke_translate_zh_en() {
        let dir = model_dir("opus-mt-zh-en");
        if !dir.exists() {
            eprintln!("model dir missing, skipping: {}", dir.display());
            return;
        }
        let engine = OpusMtEngine::load(&dir).expect("load zh-en engine");
        let out = engine.translate("你好，世界。今天天气很好。").expect("translate zh-en");
        eprintln!("zh-en output: {}", out);
        assert!(!out.trim().is_empty());
    }

    #[test]
    fn smoke_translate_en_zh() {
        let dir = model_dir("opus-mt-en-zh");
        if !dir.exists() {
            eprintln!("model dir missing, skipping: {}", dir.display());
            return;
        }
        let engine = OpusMtEngine::load(&dir).expect("load en-zh engine");
        let out = engine
            .translate("Hello world. The weather is nice today.")
            .expect("translate en-zh");
        eprintln!("en-zh output: {}", out);
        assert!(!out.trim().is_empty());
    }

    #[test]
    fn perf_translate_long_text() {
        let dir = model_dir("opus-mt-en-zh");
        if !dir.exists() {
            eprintln!("model dir missing, skipping: {}", dir.display());
            return;
        }
        let engine = OpusMtEngine::load(&dir).expect("load en-zh engine");
        let text = "Thank you so much. I am still fired up and ready to go. First of all, I want to congratulate everyone on a hard fought victory here. A few weeks ago, no one imagined that we would accomplish what we did here tonight. No one could have imagined it. For most of this campaign, we were far behind, and we always knew our climb would be steep.";
        let start = std::time::Instant::now();
        let out = engine.translate(text).expect("beam translate long text");
        eprintln!("beam=3 long-text ({} chars, {:?}): {}", out.chars().count(), start.elapsed(), out);
        assert!(!out.trim().is_empty());
        assert!(start.elapsed().as_secs() < 90, "beam translation too slow");
    }

    #[test]
    fn beam_vs_greedy_on_hard_text() {
        let dir = model_dir("opus-mt-en-zh");
        if !dir.exists() {
            eprintln!("model dir missing, skipping: {}", dir.display());
            return;
        }
        let engine = OpusMtEngine::load(&dir).expect("load en-zh engine");
        // 已知难点文本：小模型在此类复杂多句输入上可能丢句（模型能力上限，
        // 非解码 bug）。此测试用于观察两种解码的输出差异，不断言完整性。
        let text = "I am still fired up and ready to go. Thank you. Thank you. Well, first of all, I want to congratulate Senator Clinton on a hard fought victory here in New Hampshire.";
        let beam = engine.translate(text).expect("beam translate");
        let greedy = engine.translate_greedy(text).expect("greedy translate");
        eprintln!("greedy: {}", greedy);
        eprintln!("beam=3: {}", beam);
        assert!(!beam.trim().is_empty());
        assert!(!greedy.trim().is_empty());
    }
}
