use std::io::{self, BufRead, Write};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use encoding_rs;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::token::LlamaToken;
use serde::{Deserialize, Serialize};

// ============================================================================
// Protocol Messages (JSON over stdin/stdout)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    Generate {
        prompt: String,
        max_tokens: Option<i32>,
        context_size: Option<u32>,
        model_path: Option<String>,
        // Sampling parameters
        temperature: Option<f32>,
        top_k: Option<i32>,
        top_p: Option<f32>,
        repeat_penalty: Option<f32>,
        frequency_penalty: Option<f32>,
        stop_tokens: Option<Vec<String>>,
        // When true, each generated token is also emitted as a
        // `{"type":"token","text":...}` line before the final response.
        stream: Option<bool>,
    },
    Ping,
    Shutdown,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    Response { text: String, error: Option<String> },
    Token { text: String },
    Pong,
    Goodbye,
    Error { message: String },
    /// Model load progress, emitted before/after an actual (re)load so the
    /// host app can surface loading state to the UI.
    #[serde(rename = "model-loading")]
    ModelLoading {
        phase: String,
        model_path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        elapsed_ms: Option<u64>,
    },
}

// ============================================================================
// VRAM Detection and GPU Layer Calculation
// ============================================================================

/// Detect available VRAM in GB
fn detect_vram_gb() -> f32 {
    #[cfg(feature = "metal")]
    {
        // macOS Metal: Query recommended max working set size
        if let Some(vram) = detect_metal_vram() {
            eprintln!("Metal VRAM detected: {:.2} GB", vram);
            return vram;
        }
    }

    #[cfg(feature = "cuda")]
    {
        // NVIDIA CUDA: Query device memory
        if let Some(vram) = detect_cuda_vram() {
            eprintln!("CUDA VRAM detected: {:.2} GB", vram);
            return vram;
        }
    }

    /// TODO: Vulkan VRAM detection

    eprintln!("VRAM detection not available, using conservative estimate");
    4.0 // Conservative fallback
}

#[cfg(feature = "metal")]
fn detect_metal_vram() -> Option<f32> {
    if let Ok(output) = std::process::Command::new("sysctl")
        .arg("hw.memsize")
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            if let Some(bytes_str) = stdout.split(':').nth(1) {
                if let Ok(bytes) = bytes_str.trim().parse::<u64>() {
                    let gb = bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    // Assume GPU can use ~60% of system memory on Apple Silicon
                    return Some(gb * 0.6);
                }
            }
        }
    }
    None
}

#[cfg(feature = "cuda")]
fn detect_cuda_vram() -> Option<f32> {
    // Use nvidia-smi to query VRAM
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args(&["--query-gpu=memory.free", "--format=csv,noheader,nounits"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            if let Ok(mb) = stdout.trim().parse::<f32>() {
                return Some(mb / 1024.0); // Convert MB to GB
            }
        }
    }
    None
}

/// Calculate safe GPU layer count based on VRAM, model file size, and context size
fn calculate_gpu_layers(
    model_path: &PathBuf,
    model_layers: u32,
    vram_gb: f32,
    context_size: u32,
) -> u32 {
    let file_size_gb = std::fs::metadata(model_path)
        .map(|m| m.len() as f32 / 1024.0 / 1024.0 / 1024.0)
        .unwrap_or(0.0);

    if file_size_gb == 0.0 {
        eprintln!("⚠️ Could not determine model file size, using conservative default");
        return 0;
    }

    // Heuristic: Estimate KV cache size
    // 7B models (approx > 2.5GB) usually have 4096 hidden dim -> ~256MB per 1k context
    // 1B models (approx < 2.5GB) usually have 2048 hidden dim -> ~128MB per 1k context
    let kv_per_1k_gb = if file_size_gb > 2.5 { 0.25 } else { 0.12 };
    let total_kv_gb = (context_size as f32 / 1000.0) * kv_per_1k_gb;

    // Safety buffer (500MB) for OS/Display
    let safe_vram = vram_gb - 0.5;

    // For debugging
    eprintln!("📊 VRAM Analysis:");
    eprintln!("   • Available: {:.2} GB", vram_gb);
    eprintln!("   • Safe Limit: {:.2} GB", safe_vram);
    eprintln!("   • Model Weights: {:.2} GB", file_size_gb);
    eprintln!(
        "   • KV Cache ({} ctx): {:.2} GB",
        context_size, total_kv_gb
    );

    if safe_vram <= 0.0 {
        eprintln!("⚠️ No safe VRAM available, using CPU only");
        return 0;
    }

    // Calculate cost per layer
    let weight_per_layer = file_size_gb / model_layers as f32;
    let kv_per_layer = total_kv_gb / model_layers as f32;
    let total_per_layer = weight_per_layer + kv_per_layer;

    // Calculate how many layers fit
    let safe_layers = (safe_vram / total_per_layer).floor() as u32;
    let layers = safe_layers.min(model_layers);

    eprintln!(
        "   • Cost per layer: {:.2} MB (Weights) + {:.2} MB (KV) = {:.2} MB",
        weight_per_layer * 1024.0,
        kv_per_layer * 1024.0,
        total_per_layer * 1024.0
    );

    if layers < model_layers {
        eprintln!(
            "⚠️ Memory constrained. Offloading {}/{} layers ({:.1}%)",
            layers,
            model_layers,
            (layers as f32 / model_layers as f32) * 100.0
        );
    } else {
        eprintln!("✅ Full offload possible ({} layers)", layers);
    }

    layers
}

/// Get default GPU layer count with smart detection
fn get_default_gpu_layers(model_path: &PathBuf, context_size: u32) -> u32 {
    let vram = detect_vram_gb();
    // TODO: Use actual model metadata instead of heuristics
    // Heuristic: Estimate total layers based on file size
    // 7B models (Q4) are ~4.1GB and have ~32-35 layers
    // 1B models (Q4) are ~1.1GB and have ~20-28 layers
    let file_size_gb = std::fs::metadata(model_path)
        .map(|m| m.len() as f32 / 1024.0 / 1024.0 / 1024.0)
        .unwrap_or(0.0);

    let estimated_layers = if file_size_gb > 2.5 { 33 } else { 28 };

    calculate_gpu_layers(model_path, estimated_layers, vram, context_size)
}

// ============================================================================
// Model State Management
// ============================================================================

struct ModelState {
    backend: LlamaBackend,
    // Persistent context (created lazily on first generate) so the KV cache
    // survives between requests and shared prompt prefixes can be reused.
    // Declared before `model` so it is dropped first.
    ctx: Option<LlamaContext<'static>>,
    // Full token list of the previous prompt, mirroring KV cache positions
    // [0, len) of seq 0; used to compute the reusable prefix.
    prev_prompt_tokens: Vec<LlamaToken>,
    model: Option<LlamaModel>,
    model_path: Option<PathBuf>,
    context_size: u32,
    last_activity: Arc<AtomicU64>,
}

impl ModelState {
    fn new() -> Result<Self> {
        let backend = LlamaBackend::init().context("Failed to init LlamaBackend")?;
        Ok(Self {
            backend,
            ctx: None,
            prev_prompt_tokens: Vec::new(),
            model: None,
            model_path: None,
            context_size: 2048,
            last_activity: Arc::new(AtomicU64::new(Self::current_timestamp())),
        })
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn update_activity(&self) {
        self.last_activity
            .store(Self::current_timestamp(), Ordering::SeqCst);
    }

    fn seconds_since_activity(&self) -> u64 {
        Self::current_timestamp() - self.last_activity.load(Ordering::SeqCst)
    }

    fn load_model_if_needed(&mut self, model_path: PathBuf, context_size: u32) -> Result<()> {
        // Check if model is already loaded
        if let Some(ref loaded_path) = self.model_path {
            if loaded_path == &model_path && self.context_size == context_size {
                eprintln!("✓ Model already loaded");
                self.update_activity();
                return Ok(());
            }
        }

        eprintln!("📥 Loading model: {}", model_path.display());
        let load_start = Instant::now();
        send_response(&Response::ModelLoading {
            phase: "start".to_string(),
            model_path: model_path.to_string_lossy().to_string(),
            elapsed_ms: None,
        })?;

        // Model or context size changed: the persistent context and its KV
        // cache are no longer valid. Drop the context BEFORE replacing the
        // model it borrows from.
        self.ctx = None;
        self.prev_prompt_tokens.clear();

        // Detect GPU layers
        let gpu_layers = get_default_gpu_layers(&model_path, context_size);

        // Configure model parameters with GPU offload
        let model_params = LlamaModelParams::default().with_n_gpu_layers(gpu_layers);
        let model_params = pin!(model_params);

        let model = LlamaModel::load_from_file(&self.backend, model_path.clone(), &model_params)
            .with_context(|| format!("unable to load model at {:?}", model_path))?;

        let elapsed_ms = load_start.elapsed().as_millis() as u64;
        send_response(&Response::ModelLoading {
            phase: "done".to_string(),
            model_path: model_path.to_string_lossy().to_string(),
            elapsed_ms: Some(elapsed_ms),
        })?;

        self.model = Some(model);
        self.model_path = Some(model_path);
        self.context_size = context_size;
        self.update_activity();

        eprintln!("✅ Model loaded successfully");
        Ok(())
    }

    fn generate(
        &mut self,
        prompt: String,
        max_tokens: i32,
        temperature: f32,
        top_k: i32,
        top_p: f32,
        repeat_penalty: Option<f32>,
        frequency_penalty: Option<f32>,
        stop_tokens: Vec<String>,
        stream: bool,
    ) -> Result<String> {
        let start_time = Instant::now();
        let model = self.model.as_ref().context("Model not loaded")?;

        // Calculate thread count (conservative default: max(1, (Cores / 2) + 2))
        // This ensures the UI thread is never starved
        let threads: i32 = std::thread::available_parallelism()
            .map(|n| {
                let cores = n.get() as i32;
                ((cores / 2) + 2).max(1)
            })
            .unwrap_or(2);

        // Persistent context, created once per model/context_size so the KV
        // cache can be partially retained across requests (prefix reuse).
        if self.ctx.is_none() {
            let ctx_params = LlamaContextParams::default()
                .with_n_ctx(Some(
                    NonZeroU32::new(self.context_size).context("Invalid ctx size")?,
                ))
                .with_n_batch(self.context_size)
                .with_n_threads(threads)
                .with_n_threads_batch(threads);

            let ctx = model
                .new_context(&self.backend, ctx_params)
                .context("unable to create the llama_context")?;
            // SAFETY: LlamaModel is a NonNull wrapper whose pointee lives in a
            // stable llama.cpp allocation, so the borrow inside LlamaContext
            // stays valid while the model is alive. The context is declared
            // before `model` in ModelState (dropped first) and explicitly
            // cleared in load_model_if_needed before any model replacement.
            let ctx: LlamaContext<'static> = unsafe { std::mem::transmute(ctx) };
            self.ctx = Some(ctx);
        }

        let tokens_list = model
            .str_to_token(&prompt, AddBos::Always)
            .with_context(|| "failed to tokenize prompt")?;

        eprintln!("📝 Tokenized prompt: {} tokens", tokens_list.len());

        // Longest common prefix with the previous prompt. KV positions
        // [0, hit) of seq 0 are still valid and can be skipped.
        let mut common = 0usize;
        while common < tokens_list.len()
            && common < self.prev_prompt_tokens.len()
            && tokens_list[common] == self.prev_prompt_tokens[common]
        {
            common += 1;
        }
        // Require a meaningful hit, and always re-decode at least the final
        // prompt token so its logits are available for the first sample.
        let prefix_hit = if common >= 16 && !self.prev_prompt_tokens.is_empty() {
            common.min(tokens_list.len() - 1)
        } else {
            0
        };

        let ctx = self.ctx.as_mut().expect("context ensured above");
        if prefix_hit > 0 {
            // Trim seq 0 KV in [prefix_hit, end); the cached prefix stays.
            ctx.clear_kv_cache_seq(Some(0), Some(prefix_hit as u32), None)
                .context("failed to trim KV cache")?;
            eprintln!(
                "♻️ Prefix reuse: {}/{} tokens cached",
                prefix_hit,
                tokens_list.len()
            );
        } else {
            // Divergent prompt: start from a clean KV cache.
            ctx.clear_kv_cache();
            if !self.prev_prompt_tokens.is_empty() {
                eprintln!(
                    "♻️ Prefix reuse: 0/{} tokens cached (prefix mismatch)",
                    tokens_list.len()
                );
            }
        }

        // Use context size for batch capacity to handle long prompts
        let batch_size = self.context_size as usize;
        let mut batch = LlamaBatch::new(batch_size, 1);

        // Only the suffix [prefix_hit, len) needs decoding; positions are
        // absolute so they line up with the retained KV cache entries.
        let last_index = tokens_list.len() - 1;
        for (i, token) in tokens_list.iter().enumerate().skip(prefix_hit) {
            batch
                .add(*token, i as i32, &[0], i == last_index)
                .context("Failed to add token to batch")?;
        }

        ctx.decode(&mut batch).context("llama_decode() failed")?;
        let prompt_time = start_time.elapsed();

        // Absolute position after the prompt (generation bookkeeping).
        let n_prompt_tokens = tokens_list.len() as i32;
        let mut n_cur = n_prompt_tokens;
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut output = String::new();

        use llama_cpp_2::sampling::LlamaSampler;

        // Build the sampler chain once per request; penalties keep their
        // accepted-token history via `sampler.accept(token)` below.
        let sampler = if temperature <= 0.0 {
            // Greedy sampling for temp <= 0
            LlamaSampler::chain_simple([LlamaSampler::greedy()])
        } else {
            // Random sampling with temperature/top_k/top_p (+ optional penalties)
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u32;

            let mut samplers = Vec::new();
            if repeat_penalty.is_some() || frequency_penalty.is_some() {
                // penalty_last_n = -1 (whole context); presence penalty unused
                samplers.push(LlamaSampler::penalties(
                    -1,
                    repeat_penalty.unwrap_or(1.0),
                    frequency_penalty.unwrap_or(0.0),
                    0.0,
                ));
            }
            samplers.push(LlamaSampler::top_k(top_k));
            samplers.push(LlamaSampler::top_p(top_p, 1));
            samplers.push(LlamaSampler::temp(temperature));
            samplers.push(LlamaSampler::dist(seed));
            LlamaSampler::chain_simple(samplers)
        };
        let mut sampler = pin!(sampler);

        eprintln!("🔄 Starting generation (max_tokens: {})", max_tokens);

        loop {
            // Check if we've generated enough tokens
            if (n_cur - n_prompt_tokens) >= max_tokens {
                eprintln!("✓ Reached max_tokens limit");
                break;
            }

            // Sample from the logits of the batch's last token (the final
            // prompt token on the first step, the generated token afterwards).
            let token = sampler.as_mut().sample(ctx, batch.n_tokens() - 1);
            sampler.as_mut().accept(token);

            if model.is_eog_token(token) {
                eprintln!(
                    "✓ End-of-generation token reached (generated {} chars)",
                    output.len()
                );
                break;
            }

            let output_bytes = model
                .token_to_bytes(token, Special::Tokenize)
                .context("Failed to convert token to bytes")?;

            let mut token_text = String::with_capacity(32);
            let _ = decoder.decode_to_string(&output_bytes, &mut token_text, false);
            output.push_str(&token_text);

            if stream && !token_text.is_empty() {
                send_response(&Response::Token {
                    text: token_text,
                })?;
            }

            // Check for model-specific stop tokens
            let mut should_stop = false;
            for stop_token in &stop_tokens {
                if output.contains(stop_token) {
                    eprintln!(
                        "✓ Stop token '{}' detected (generated {} chars)",
                        stop_token,
                        output.len()
                    );
                    // Remove the stop token from output
                    output = output.replace(stop_token, "").trim_end().to_string();
                    should_stop = true;
                    break;
                }
            }
            if should_stop {
                break;
            }

            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .context("Failed to add generated token to batch")?;
            n_cur += 1;
            ctx.decode(&mut batch).context("failed to eval")?;
        }

        // Remember the full prompt for the next request's prefix reuse.
        self.prev_prompt_tokens = tokens_list;

        // Generation statistics
        let total_time = start_time.elapsed();
        let gen_time = total_time.saturating_sub(prompt_time);
        let output_tokens = (n_cur - n_prompt_tokens) as u64;
        let prompt_tokens = n_prompt_tokens as u64;

        let tokens_per_sec = if gen_time.as_secs_f64() > 0.0 {
            output_tokens as f64 / gen_time.as_secs_f64()
        } else {
            0.0
        };

        eprintln!("📊 Generation Statistics:");
        eprintln!("   • Prompt tokens: {}", prompt_tokens);
        eprintln!("   • Output tokens: {}", output_tokens);
        eprintln!("   • Prompt processing: {:.2}s", prompt_time.as_secs_f64());
        eprintln!("   • Generation time: {:.2}s", gen_time.as_secs_f64());
        eprintln!("   • Total time: {:.2}s", total_time.as_secs_f64());
        eprintln!("   • Speed: {:.2} tokens/sec", tokens_per_sec);

        self.update_activity();
        Ok(output)
    }
}

// ============================================================================
// Main Loop with Keep-Alive Protocol
// ============================================================================

fn send_response(response: &Response) -> Result<()> {
    let json = serde_json::to_string(response)?;
    println!("{}", json);
    io::stdout().flush()?;
    Ok(())
}

fn main() -> Result<()> {
    // Get idle timeout from environment variable (default 30 minutes)
    let idle_timeout_secs = std::env::var("LLAMA_IDLE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1800); // 30 minutes default

    eprintln!(
        "🦙 llama-helper starting (idle timeout: {}s)",
        idle_timeout_secs
    );

    let mut state = ModelState::new()?;

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut buffer = String::new();

    loop {
        // Check idle timeout
        if state.seconds_since_activity() > idle_timeout_secs {
            eprintln!("💤 Idle timeout reached, shutting down");
            send_response(&Response::Goodbye)?;
            break;
        }

        // Read line from stdin
        buffer.clear();
        match stdin_lock.read_line(&mut buffer) {
            Ok(0) => {
                // EOF reached
                eprintln!("📪 EOF received, shutting down");
                break;
            }
            Ok(_) => {
                let line = buffer.trim();
                if line.is_empty() {
                    continue;
                }

                // Parse request
                match serde_json::from_str::<Request>(line) {
                    Ok(Request::Generate {
                        prompt,
                        max_tokens,
                        context_size,
                        model_path,
                        temperature,
                        top_k,
                        top_p,
                        repeat_penalty,
                        frequency_penalty,
                        stop_tokens,
                        stream,
                    }) => {
                        let max_tokens = max_tokens.unwrap_or(512);
                        let context_size = context_size.unwrap_or(2048);

                        // Sampling parameters with sensible defaults
                        let temperature = temperature.unwrap_or(1.0);
                        let top_k = top_k.unwrap_or(64);
                        let top_p = top_p.unwrap_or(0.95);
                        let stop_tokens = stop_tokens.unwrap_or_else(Vec::new);
                        let stream = stream.unwrap_or(false);

                        // Load model if path provided
                        if let Some(path_str) = model_path {
                            let path = PathBuf::from(path_str);
                            if let Err(e) = state.load_model_if_needed(path, context_size) {
                                send_response(&Response::Response {
                                    text: String::new(),
                                    error: Some(format!("Failed to load model: {}", e)),
                                })?;
                                continue;
                            }
                        }

                        // Generate response with sampling parameters
                        match state.generate(
                            prompt,
                            max_tokens,
                            temperature,
                            top_k,
                            top_p,
                            repeat_penalty,
                            frequency_penalty,
                            stop_tokens,
                            stream,
                        ) {
                            Ok(text) => {
                                send_response(&Response::Response { text, error: None })?;
                            }
                            Err(e) => {
                                send_response(&Response::Response {
                                    text: String::new(),
                                    error: Some(format!("Generation failed: {}", e)),
                                })?;
                            }
                        }
                    }
                    Ok(Request::Ping) => {
                        state.update_activity();
                        send_response(&Response::Pong)?;
                    }
                    Ok(Request::Shutdown) => {
                        eprintln!("🛑 Shutdown requested");
                        send_response(&Response::Goodbye)?;
                        break;
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to parse request: {}", e);
                        send_response(&Response::Error {
                            message: format!("Invalid request: {}", e),
                        })?;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error reading stdin: {}", e);
                break;
            }
        }
    }

    eprintln!("👋 llama-helper exiting");
    Ok(())
}
