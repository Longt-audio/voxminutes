use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use silero_rs::{VadConfig, VadSession, VadTransition};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Represents a complete speech segment detected by VAD
#[derive(Debug, Clone)]
pub struct SpeechSegment {
    pub samples: Vec<f32>,
    pub start_timestamp_ms: f64,
    pub end_timestamp_ms: f64,
    pub confidence: f32,
}

/// Processes audio in 30ms chunks but returns complete speech segments
pub struct ContinuousVadProcessor {
    session: VadSession,
    chunk_size: usize,
    sample_rate: u32,
    buffer: Vec<f32>,
    speech_segments: VecDeque<SpeechSegment>,
    current_speech: Vec<f32>,
    in_speech: bool,
    processed_samples: usize,
    speech_start_sample: usize,
    // 本轮语音是否发生过 15s 强制切段：切段后 silero 会话内部仍缓存整段音频，
    // 自然断句时 SpeechEnd 会带回「整段」samples（重复转写），此时必须改用切段后
    // 重新累积的 current_speech 尾部（见 process_chunk 的 SpeechEnd 分支）。
    force_split_active: bool,
    // State tracking for smart logging
    last_logged_state: bool,
    // Rate-limit the "large speech buffer" warning to avoid log spam.
    last_large_buffer_warn: Option<Instant>,
}

impl ContinuousVadProcessor {
    pub fn new(input_sample_rate: u32, redemption_time_ms: u32) -> Result<Self> {
        // VAD parameters aligned with voxminutes-main--reference for reliable real-time detection.
        // Slightly lower thresholds for meeting recordings so quiet starts / short
        // confirmations (e.g. "嗯", "啊") are not dropped by offline VAD.
        Self::new_with_thresholds(input_sample_rate, redemption_time_ms, 0.45, 0.30)
    }

    /// Same as `new` but with explicit silero speech thresholds, for offline
    /// processing where a more sensitive pass may be tuned per use case.
    pub fn new_with_thresholds(
        input_sample_rate: u32,
        redemption_time_ms: u32,
        positive_speech_threshold: f32,
        negative_speech_threshold: f32,
    ) -> Result<Self> {
        // Silero VAD MUST use 16kHz - this is hardcoded requirement
        const VAD_SAMPLE_RATE: u32 = 16000;

        // Use STRICT settings to prevent silence from reaching Whisper
        let mut config = VadConfig::default();
        config.sample_rate = VAD_SAMPLE_RATE as usize;

        // Reference uses Silero defaults with 400ms redemption_time on all platforms,
        // which satisfies post_speech_pad (400ms) ≤ redemption_time (400ms) constraint
        // and prevents "Duration Xms is outside of session audio range" panics.
        config.positive_speech_threshold = positive_speech_threshold; // default 0.50
        config.negative_speech_threshold = negative_speech_threshold; // default 0.35, prevents VAD state sticking
        config.redemption_time = Duration::from_millis(redemption_time_ms as u64);
        // Guard: callers may pass very short redemption times; keep a reasonable
        // floor to avoid chopping normal pauses inside sentences.
        if config.redemption_time < Duration::from_millis(500) {
            config.redemption_time = Duration::from_millis(500);
        }
        config.pre_speech_pad = Duration::from_millis(300); // Pre-speech padding for context
        config.post_speech_pad = Duration::from_millis(400); // Post-speech padding (≤ redemption_time)

        // CRITICAL FIX: Increased min_speech_time to prevent tiny 40ms fragments
        // Previous: 100ms allowed too-short segments that Whisper rejects
        // New: 250ms ensures segments are substantial enough for Whisper (>100ms requirement)
        config.min_speech_time = Duration::from_millis(250); // Prevent tiny fragments

        debug!("Creating VAD session with: sample_rate={}Hz, redemption={}ms, min_speech={}ms, input_rate={}Hz",
               VAD_SAMPLE_RATE, redemption_time_ms, 250, input_sample_rate);

        let session = VadSession::new(config)
            .map_err(|e| anyhow!("Failed to create VAD session: {:?}", e))?;

        // VAD uses 30ms chunks at 16kHz (480 samples)
        let vad_chunk_size = (VAD_SAMPLE_RATE as f32 * 0.03) as usize; // 480 samples

        info!(
            "VAD processor created: input={}Hz, vad={}Hz, chunk_size={} samples",
            input_sample_rate, VAD_SAMPLE_RATE, vad_chunk_size
        );

        Ok(Self {
            session,
            chunk_size: vad_chunk_size,
            sample_rate: input_sample_rate, // Store input rate for resampling ratio in resample_to_16k()
            buffer: Vec::with_capacity(vad_chunk_size * 2),
            speech_segments: VecDeque::new(),
            current_speech: Vec::new(),
            in_speech: false,
            processed_samples: 0,
            speech_start_sample: 0,
            force_split_active: false,
            // Initialize state tracking
            last_logged_state: false,
            last_large_buffer_warn: None,
        })
    }

    /// 当前是否处于「语音进行中」（VAD 已检测到人声、但尚未触发断句）。
    /// 供上层向前端上报「正在识别，等待断句」状态。
    pub fn is_speaking(&self) -> bool {
        self.in_speech
    }

    /// Process incoming audio samples and return any complete speech segments
    /// Handles resampling from input sample rate to 16kHz for VAD processing
    pub fn process_audio(&mut self, samples: &[f32]) -> Result<Vec<SpeechSegment>> {
        // Defensive: upstream processing (AGC / loudness normalization) can emit
        // samples outside [-1.0, 1.0], which silero rejects with an error.
        // Clamp them here so one out-of-range chunk doesn't drop VAD frames.
        let clamped_samples;
        let samples = if samples
            .iter()
            .any(|s| !(-1.0..=1.0).contains(s) || s.is_nan())
        {
            clamped_samples = samples
                .iter()
                .map(|s| if s.is_nan() { 0.0 } else { s.clamp(-1.0, 1.0) })
                .collect::<Vec<f32>>();
            log::debug!("VAD: clamped out-of-range samples");
            &clamped_samples
        } else {
            samples
        };

        // Resample to 16kHz if needed
        let resampled_audio = if self.sample_rate == 16000 {
            samples.to_vec()
        } else {
            self.resample_to_16k(samples)?
        };

        self.buffer.extend_from_slice(&resampled_audio);
        let mut completed_segments = Vec::new();

        // Process complete 30ms chunks (480 samples at 16kHz)
        while self.buffer.len() >= self.chunk_size {
            let chunk: Vec<f32> = self.buffer.drain(..self.chunk_size).collect();
            self.process_chunk(&chunk)?;

            // Extract any completed speech segments
            while let Some(segment) = self.speech_segments.pop_front() {
                completed_segments.push(segment);
            }
        }

        Ok(completed_segments)
    }

    /// Improved resampling from input sample rate to 16kHz with anti-aliasing
    /// Uses linear interpolation and basic low-pass filtering for better quality
    fn resample_to_16k(&self, samples: &[f32]) -> Result<Vec<f32>> {
        if self.sample_rate == 16000 {
            return Ok(samples.to_vec());
        }

        // Calculate downsampling ratio
        let ratio = self.sample_rate as f64 / 16000.0;
        let output_len = (samples.len() as f64 / ratio) as usize;
        let mut resampled = Vec::with_capacity(output_len);

        // Apply simple low-pass filter before downsampling to reduce aliasing
        let cutoff_freq = 0.4; // Normalized frequency (0.4 * Nyquist)
        let mut filtered_samples = Vec::with_capacity(samples.len());

        // Simple moving average filter (basic low-pass)
        let filter_size =
            (self.sample_rate as f64 / (cutoff_freq * self.sample_rate as f64)) as usize;
        let filter_size = std::cmp::max(1, std::cmp::min(filter_size, 5)); // Limit filter size

        for i in 0..samples.len() {
            let start = if i >= filter_size { i - filter_size } else { 0 };
            let end = std::cmp::min(i + filter_size + 1, samples.len());
            let sum: f32 = samples[start..end].iter().sum();
            filtered_samples.push(sum / (end - start) as f32);
        }

        // Linear interpolation downsampling
        for i in 0..output_len {
            let source_pos = i as f64 * ratio;
            let source_index = source_pos as usize;
            let fraction = source_pos - source_index as f64;

            if source_index + 1 < filtered_samples.len() {
                // Linear interpolation
                let sample1 = filtered_samples[source_index];
                let sample2 = filtered_samples[source_index + 1];
                let interpolated = sample1 + (sample2 - sample1) * fraction as f32;
                resampled.push(interpolated);
            } else if source_index < filtered_samples.len() {
                resampled.push(filtered_samples[source_index]);
            }
        }

        debug!(
            "Resampled from {} samples ({}Hz) to {} samples (16kHz) with anti-aliasing",
            samples.len(),
            self.sample_rate,
            resampled.len()
        );

        Ok(resampled)
    }

    /// Flush any remaining audio and return final speech segments
    pub fn flush(&mut self) -> Result<Vec<SpeechSegment>> {
        debug!("VAD flush: in_speech={}, current_speech_len={}, buffer_len={}, speech_segments_queued={}",
              self.in_speech, self.current_speech.len(), self.buffer.len(), self.speech_segments.len());

        let mut completed_segments = Vec::new();

        // Process any remaining buffered audio
        if !self.buffer.is_empty() {
            let remaining = self.buffer.clone();
            self.buffer.clear();

            // Pad to chunk size if needed
            let mut padded_chunk = remaining;
            if padded_chunk.len() < self.chunk_size {
                padded_chunk.resize(self.chunk_size, 0.0);
            }

            self.process_chunk(&padded_chunk)?;
        }

        // Force end any ongoing speech
        if self.in_speech && !self.current_speech.is_empty() {
            // processed_samples and speech_start_sample always count 16kHz samples (post-resampling)
            let start_ms = (self.speech_start_sample as f64 / 16000.0) * 1000.0;
            let end_ms = (self.processed_samples as f64 / 16000.0) * 1000.0;

            debug!(
                "VAD flush: Force-ending speech - start={}ms, end={}ms, duration={}ms, samples={}",
                start_ms,
                end_ms,
                end_ms - start_ms,
                self.current_speech.len()
            );

            let segment = SpeechSegment {
                samples: self.current_speech.clone(),
                start_timestamp_ms: start_ms,
                end_timestamp_ms: end_ms,
                confidence: 0.8, // Estimated confidence for forced end
            };

            self.speech_segments.push_back(segment);
            self.current_speech.clear();
            self.in_speech = false;
        }

        // Extract all remaining segments
        while let Some(segment) = self.speech_segments.pop_front() {
            completed_segments.push(segment);
        }

        Ok(completed_segments)
    }

    fn process_chunk(&mut self, chunk: &[f32]) -> Result<()> {
        // Track accumulated speech buffer size to detect memory issues.
        // Rate-limit the warning: long continuous speech is normal in meetings,
        // but logging it every 10ms drowns out useful information.
        const LARGE_BUFFER_WARN_INTERVAL: Duration = Duration::from_secs(30);
        let current_speech_size = self.current_speech.len();
        if current_speech_size > 1_000_000 {
            let now = Instant::now();
            if self.last_large_buffer_warn.map_or(true, |t| {
                now.duration_since(t) >= LARGE_BUFFER_WARN_INTERVAL
            }) {
                warn!("VAD: Accumulated speech buffer is large: {} samples ({:.1}s) - possible memory issue",
                      current_speech_size, current_speech_size as f64 / 16000.0);
                self.last_large_buffer_warn = Some(now);
            }
        }

        let transitions = self
            .session
            .process(chunk)
            .map_err(|e| anyhow!("VAD processing failed: {}", e))?;

        // Log transitions for debugging
        if !transitions.is_empty() {
            debug!(
                "VAD transitions at sample {}: {} transitions",
                self.processed_samples,
                transitions.len()
            );
        }

        // Handle VAD transitions
        for transition in transitions {
            match transition {
                VadTransition::SpeechStart { timestamp_ms } => {
                    // Only log if state changed
                    if !self.last_logged_state {
                        debug!("VAD: Speech started at {}ms", timestamp_ms);
                        self.last_logged_state = true;
                    }
                    self.in_speech = true;
                    // Silero VAD timestamp is absolute/cumulative (ms since VAD session start),
                    // on the same axis as processed_samples — use it directly. (Previously this
                    // added processed_samples on top, double-counting and producing ~2x start
                    // times for force-ended flush segments.)
                    self.speech_start_sample = (timestamp_ms as u64 * 16000 / 1000) as usize;
                    self.current_speech.clear();
                    self.force_split_active = false;
                }
                VadTransition::SpeechEnd {
                    start_timestamp_ms,
                    end_timestamp_ms,
                    samples,
                } => {
                    // Only log if we were previously in speech state
                    if self.last_logged_state {
                        debug!(
                            "VAD: Speech ended at {}ms (duration: {}ms)",
                            end_timestamp_ms,
                            end_timestamp_ms - start_timestamp_ms
                        );
                        self.last_logged_state = false;
                    }
                    self.in_speech = false;

                    // 若本轮语音中途被 15s 强制切段过：silero 的 SpeechEnd 会带回
                    // 「整段」samples（其内部缓冲自 SpeechStart 起未清），直接使用会把
                    // 已提交部分再转录一遍（转写文本重复，见 2026-09-19 日志排查）。
                    // 此时改用切段后重新累积的尾部 current_speech，起点取 speech_start_sample
                    // （强制切段时已更新为切点），终点沿用 silero 的自然断句时间。
                    let (segment_start_ms, segment_end_ms, speech_samples) = if self.force_split_active {
                        self.force_split_active = false;
                        let start_ms = (self.speech_start_sample as f64 / 16000.0) * 1000.0;
                        let tail = std::mem::take(&mut self.current_speech);
                        // ⚠️ silero 的 end_timestamp_ms 与我们的 processed_samples 是**两条时间轴**
                        // （前者是 VAD 内部会话时钟，后者是送入音频的累计时钟，分块/重采样后会有偏差）。
                        // 强制切段后混用会出现 end < start —— 2026-09-24 用真实语音测出 **-1010ms
                        // 的负时长段**（下游按「段太短」过滤时可能连带丢掉这段音频）。
                        // 修法不是夹取，而是**终点也用自己的时钟**（与起点 speech_start_sample 同轴，
                        // 差值正好等于这段 tail 的时长）。
                        let end_ms = (self.processed_samples as f64 / 16000.0) * 1000.0;
                        (start_ms, end_ms, tail)
                    } else {
                        // Use samples from VAD transition if available, otherwise use accumulated samples
                        let speech_samples = if !samples.is_empty() {
                            samples
                        } else {
                            self.current_speech.clone()
                        };
                        (start_timestamp_ms as f64, end_timestamp_ms as f64, speech_samples)
                    };

                    if !speech_samples.is_empty() {
                        let segment = SpeechSegment {
                            samples: speech_samples,
                            start_timestamp_ms: segment_start_ms,
                            end_timestamp_ms: segment_end_ms,
                            confidence: 0.9, // VAD confidence
                        };

                        info!(
                            "VAD: Completed speech segment: {:.1}ms duration, {} samples",
                            segment_end_ms - segment_start_ms,
                            segment.samples.len()
                        );

                        self.speech_segments.push_back(segment);
                    }

                    self.current_speech.clear();
                }
            }
        }

        // Accumulate speech if we're currently in a speech state
        if self.in_speech {
            self.current_speech.extend_from_slice(chunk);
        }

        self.processed_samples += chunk.len();

        // 连续语音超长强制切分：纯靠静音边界时，持续讲话/带背景音的视频会让
        // 「当前段」一直不闭合，批量（非流式）转写直到停止录音才出字。
        // 这里给一个 15 秒上限：到顶先把已累计的部分作为完整段提交，下一段继续。
        // （current_speech / processed_samples 均为 16kHz 采样，见 flush 注释）
        const MAX_SPEECH_SAMPLES: usize = 16000 * 15;
        if self.in_speech && self.current_speech.len() >= MAX_SPEECH_SAMPLES {
            let start_ms = (self.speech_start_sample as f64 / 16000.0) * 1000.0;
            let end_ms = (self.processed_samples as f64 / 16000.0) * 1000.0;
            info!(
                "VAD: Force-splitting continuous speech at {:.1}ms (segment {:.1}ms, {} samples)",
                end_ms,
                end_ms - start_ms,
                self.current_speech.len()
            );
            let segment = SpeechSegment {
                samples: std::mem::take(&mut self.current_speech),
                start_timestamp_ms: start_ms,
                end_timestamp_ms: end_ms,
                confidence: 0.8, // 强制切分，置信度略降（与 flush 的 force-end 一致）
            };
            self.speech_segments.push_back(segment);
            // 下一段从当前位置继续累积（in_speech 保持 true）
            self.speech_start_sample = self.processed_samples;
            // 标记：本轮语音自然断句时 SpeechEnd 不能再用 silero 的整段 samples
            self.force_split_active = true;
        }

        Ok(())
    }
}

/// Legacy function for backward compatibility - now uses the optimized approach
pub fn extract_speech_16k(samples_mono_16k: &[f32]) -> Result<Vec<f32>> {
    let mut processor = ContinuousVadProcessor::new(16000, 400)?;

    // Process all audio
    let mut all_segments = processor.process_audio(samples_mono_16k)?;
    let final_segments = processor.flush()?;
    all_segments.extend(final_segments);

    // Concatenate all speech segments
    let mut result = Vec::new();
    let num_segments = all_segments.len();
    for segment in &all_segments {
        result.extend_from_slice(&segment.samples);
    }

    // Apply balanced energy filtering for very short segments
    if result.len() < 1600 {
        // Less than 100ms at 16kHz
        let input_energy: f32 =
            samples_mono_16k.iter().map(|&x| x * x).sum::<f32>() / samples_mono_16k.len() as f32;
        let rms = input_energy.sqrt();
        let peak = samples_mono_16k
            .iter()
            .map(|&x| x.abs())
            .fold(0.0f32, f32::max);

        // BALANCED FIX: Lowered thresholds to preserve quiet speech while still filtering silence
        // Previous aggressive values (0.08/0.15) were discarding valid quiet speech
        // New values (0.03/0.08) are more balanced - catch quiet speech, reject pure silence
        if rms < 0.2 || peak < 0.20 {
            info!("-----VAD detected silence/noise (RMS: {:.6}, Peak: {:.6}), skipping to prevent hallucinations-----", rms, peak);
            return Ok(Vec::new());
        } else {
            info!(
                "VAD detected speech with sufficient energy (RMS: {:.6}, Peak: {:.6})",
                rms, peak
            );
            return Ok(samples_mono_16k.to_vec());
        }
    }

    debug!(
        "VAD: Processed {} samples, extracted {} speech samples from {} segments",
        samples_mono_16k.len(),
        result.len(),
        num_segments
    );

    Ok(result)
}

/// Simple convenience function to get speech chunks from audio
/// Uses the optimized ContinuousVadProcessor with configurable redemption time
pub fn get_speech_chunks(
    samples_mono: &[f32],
    input_sample_rate: u32,
    redemption_time_ms: u32,
) -> Result<Vec<SpeechSegment>> {
    get_speech_chunks_with_progress(
        samples_mono,
        input_sample_rate,
        redemption_time_ms,
        |_, _| true,
    )
}

/// Get speech chunks with progress callback and cancellation support
/// The callback receives (progress_percent, segments_found) and returns false to cancel
pub fn get_speech_chunks_with_progress<F>(
    samples_mono: &[f32],
    input_sample_rate: u32,
    redemption_time_ms: u32,
    progress_callback: F,
) -> Result<Vec<SpeechSegment>>
where
    F: FnMut(u32, usize) -> bool,
{
    get_speech_chunks_with_config(
        samples_mono,
        input_sample_rate,
        redemption_time_ms,
        None,
        progress_callback,
    )
}

/// Get speech chunks with explicit silero thresholds (for tuned offline passes).
/// `thresholds` = (positive_speech_threshold, negative_speech_threshold); None = defaults.
pub fn get_speech_chunks_with_config<F>(
    samples_mono: &[f32],
    input_sample_rate: u32,
    redemption_time_ms: u32,
    thresholds: Option<(f32, f32)>,
    mut progress_callback: F,
) -> Result<Vec<SpeechSegment>>
where
    F: FnMut(u32, usize) -> bool,
{
    let mut processor = match thresholds {
        Some((pos, neg)) => ContinuousVadProcessor::new_with_thresholds(
            input_sample_rate,
            redemption_time_ms,
            pos,
            neg,
        )?,
        None => ContinuousVadProcessor::new(input_sample_rate, redemption_time_ms)?,
    };

    let total_samples = samples_mono.len();

    // Scale thresholds based on input sample rate so chunk duration is consistent
    let chunk_duration_sec = 10.0;
    let large_file_duration_sec = 60.0;
    let large_file_threshold = (large_file_duration_sec * input_sample_rate as f64) as usize;
    let chunk_size = (chunk_duration_sec * input_sample_rate as f64) as usize;

    // For large files (>1 minute equivalent), process in chunks with progress logging
    let mut all_segments = Vec::new();

    if total_samples > large_file_threshold {
        info!(
            "VAD: Processing large file ({} samples = {:.1}s at {}Hz), will log progress...",
            total_samples,
            total_samples as f64 / input_sample_rate as f64,
            input_sample_rate
        );

        let mut processed = 0;
        let mut last_progress = 0u32;
        let mut chunk_count = 0;
        let total_chunks = (total_samples + chunk_size - 1) / chunk_size;

        for chunk in samples_mono.chunks(chunk_size) {
            chunk_count += 1;

            let start_time = std::time::Instant::now();
            let segments = processor.process_audio(chunk)?;
            let elapsed = start_time.elapsed();

            // Debug log for chunk processing details
            debug!(
                "VAD: Chunk {}/{} processed in {:?}, found {} segments",
                chunk_count,
                total_chunks,
                elapsed,
                segments.len()
            );

            // Warn if chunk processing took too long (>1 second)
            if elapsed.as_secs() > 1 {
                warn!(
                    "VAD: Chunk {} took {:?} - possible performance issue",
                    chunk_count, elapsed
                );
            }

            all_segments.extend(segments);

            processed += chunk.len();
            let progress = ((processed * 100) / total_samples) as u32;

            // Call progress callback every 5%
            if progress >= last_progress + 5 {
                debug!(
                    "VAD: Progress {}% ({} segments found so far)",
                    progress,
                    all_segments.len()
                );

                // Check for cancellation
                if !progress_callback(progress, all_segments.len()) {
                    info!("VAD: Cancelled by callback at {}%", progress);
                    return Err(anyhow!("VAD processing cancelled"));
                }

                last_progress = progress;
            }
        }

        let final_segments = processor.flush()?;
        all_segments.extend(final_segments);

        info!(
            "VAD: Complete! Found {} speech segments",
            all_segments.len()
        );
    } else {
        // Small file - process all at once
        all_segments = processor.process_audio(samples_mono)?;
        let final_segments = processor.flush()?;
        all_segments.extend(final_segments);
    }

    Ok(all_segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 读仓库里的真实语音样本（`src-tauri/example_audio.wav`，10s/24kHz/单声道 PCM16）。
    ///
    /// ⚠️ 为什么不再用合成正弦音（2026-09-24 排查）：silero VAD 是**训练出来的语音检测器**，
    /// 合成谐波音在它眼里就是非语音 —— 旧夹具下 VAD 什么也不报，测试却还在断言「≥6 段」，
    /// 于是长期红灯（也会掩盖真实回归）。真实语音样本才能测出分段行为。
    fn load_example_speech() -> Option<(Vec<f32>, u32)> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("example_audio.wav");
        let b = std::fs::read(path).ok()?;
        if b.len() < 44 || &b[0..4] != b"RIFF" {
            return None;
        }
        let mut pos = 12usize;
        let mut rate = 24_000u32;
        while pos + 8 <= b.len() {
            let id = &b[pos..pos + 4];
            let size = u32::from_le_bytes([b[pos + 4], b[pos + 5], b[pos + 6], b[pos + 7]]) as usize;
            let body_end = (pos + 8 + size).min(b.len());
            let body = &b[pos + 8..body_end];
            if id == b"fmt " && body.len() >= 16 {
                let channels = u16::from_le_bytes([body[2], body[3]]);
                rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
                let bits = u16::from_le_bytes([body[14], body[15]]);
                if channels != 1 || bits != 16 {
                    return None; // 夹具格式变了就让测试失败得明明白白
                }
            } else if id == b"data" {
                let samples: Vec<f32> = body
                    .chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                    .collect();
                return Some((samples, rate));
            }
            pos += 8 + size + (size % 2);
        }
        None
    }

    /// 把真实语音拼成长音频（每段之间 1s 静音），用于触发大文件分块路径。
    fn generate_long_speech_audio(total_secs: f32) -> (Vec<f32>, u32) {
        let (speech, rate) = load_example_speech().expect("example_audio.wav 应可读且为 24k 单声道 PCM16");
        let gap = vec![0.0f32; (rate as f32 * 1.0) as usize];
        let mut out: Vec<f32> = Vec::new();
        while (out.len() as f32 / rate as f32) < total_secs {
            out.extend_from_slice(&speech);
            out.extend_from_slice(&gap);
        }
        (out, rate)
    }

    #[test]
    fn test_vad_chunked_vs_single_processing() {
        // 真实语音拼到 60s（合成正弦音 silero 判为非语音，见 generate_long_speech_audio 注释）
        let (audio, rate) = generate_long_speech_audio(60.0);
        println!(
            "Generated {} samples ({:.1}s)",
            audio.len(),
            audio.len() as f32 / 16000.0
        );

        // Process all at once (like small files)
        let segments_single =
            get_speech_chunks(&audio, 16000, 2000).expect("Single processing failed");

        // Process in chunks (like large files)
        let segments_chunked =
            get_speech_chunks_with_progress(&audio, 16000, 2000, |progress, segments| {
                println!("Chunked progress: {}%, {} segments", progress, segments);
                true // Don't cancel
            })
            .expect("Chunked processing failed");
        println!(
            "Chunked processing found {} segments",
            segments_chunked.len()
        );

        // Both should find the same number of segments (approximately)
        // Allow some variance due to chunk boundary effects
        let diff = (segments_single.len() as i32 - segments_chunked.len() as i32).abs();
        assert!(
            diff <= 1,
            "Chunked and single processing found different segment counts: {} vs {} (diff: {})",
            segments_single.len(),
            segments_chunked.len(),
            diff
        );
    }

    #[test]
    fn test_vad_large_file_progress() {
        // 真实语音拼到 120s —— 触发大文件（>60s）分块路径
        let (audio, rate) = generate_long_speech_audio(120.0);
        let total_samples = audio.len();
        println!(
            "Generated {} samples ({:.1}s)",
            total_samples,
            total_samples as f32 / 16000.0
        );

        // This should trigger the large file path (>960,000 samples)
        assert!(
            total_samples > 960_000,
            "Audio should be large enough to trigger chunked processing"
        );

        let mut progress_updates = Vec::new();
        let segments =
            get_speech_chunks_with_progress(&audio, 16000, 2000, |progress, segments| {
                progress_updates.push((progress, segments));
                true // Don't cancel
            })
            .expect("Processing failed");

        println!(
            "Found {} segments with {} progress updates",
            segments.len(),
            progress_updates.len()
        );

        // Should have found multiple speech segments (one every 10 seconds)
        // 120 seconds / 10 second interval = 12 expected speech bursts
        assert!(
            segments.len() >= 6,
            "Expected at least 6 speech segments, found {}",
            segments.len()
        );

        // Should have received progress updates
        assert!(
            !progress_updates.is_empty(),
            "Expected progress updates for large file"
        );
    }

    #[test]
    fn test_vad_cancellation() {
        let (audio, _rate) = generate_long_speech_audio(120.0);

        // Cancel at 50%
        let result = get_speech_chunks_with_progress(&audio, 16000, 2000, |progress, _| {
            progress < 50 // Cancel when reaching 50%
        });

        // Should return error due to cancellation
        assert!(result.is_err(), "Expected cancellation error");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("cancelled"),
            "Error should mention cancellation: {}",
            err_msg
        );
    }

    #[test]
    fn test_vad_continuous_processor_state_across_chunks() {
        // Test that VAD state is correctly maintained across chunk boundaries
        let mut processor =
            ContinuousVadProcessor::new(16000, 2000).expect("Failed to create processor");

        // Generate audio with a speech segment that spans a chunk boundary
        let chunk_size = 160_000; // 10 seconds
        let (audio, _rate) = generate_long_speech_audio(30.0);

        // Process in 10-second chunks
        let mut all_segments = Vec::new();
        for (i, chunk) in audio.chunks(chunk_size).enumerate() {
            let segments = processor.process_audio(chunk).expect("Processing failed");
            println!(
                "Chunk {}: processed {} samples, found {} segments",
                i,
                chunk.len(),
                segments.len()
            );
            all_segments.extend(segments);
        }

        // Flush remaining
        let final_segments = processor.flush().expect("Flush failed");
        all_segments.extend(final_segments);

        println!("Total segments found: {}", all_segments.len());

        // Should find speech segments
        assert!(
            all_segments.len() >= 1,
            "Expected at least 1 speech segment"
        );
    }

    /// 不变式：任何 VAD 段的 end 都不得早于 start（2026-09-24：真实语音测出过 -1010ms）。
    #[test]
    fn vad_segments_never_have_negative_duration() {
        let (audio, _rate) = generate_long_speech_audio(70.0);
        for redemption in [400u32, 2000u32] {
            let segments =
                get_speech_chunks(&audio, 16000, redemption).expect("VAD 处理失败");
            for (i, seg) in segments.iter().enumerate() {
                assert!(
                    seg.end_timestamp_ms >= seg.start_timestamp_ms,
                    "redemption={}ms 第 {} 段时长为负：{:.1} → {:.1}",
                    redemption,
                    i,
                    seg.start_timestamp_ms,
                    seg.end_timestamp_ms
                );
            }
        }
    }

    #[test]
    fn test_vad_400ms_vs_2000ms_segmentation() {
        // Demonstrates why 2000ms redemption is needed for batch processing:
        // 400ms creates excessive fragmentation, 2000ms bridges natural pauses.
        //
        // 真实语音拼到 60s（每段之间 1s 静音）
        let (audio, _rate) = generate_long_speech_audio(60.0);

        let segments_400 = get_speech_chunks(&audio, 16000, 400).expect("400ms processing failed");
        let segments_2000 =
            get_speech_chunks(&audio, 16000, 2000).expect("2000ms processing failed");

        println!(
            "400ms redemption: {} segments, 2000ms redemption: {} segments",
            segments_400.len(),
            segments_2000.len()
        );

        // 2000ms should produce fewer or equal segments (bridges more pauses)
        assert!(
            segments_2000.len() <= segments_400.len(),
            "2000ms redemption ({} segments) should not produce more segments than 400ms ({} segments)",
            segments_2000.len(),
            segments_400.len()
        );

        // Verify segments have reasonable durations with 2000ms
        for (i, seg) in segments_2000.iter().enumerate() {
            let duration_ms = seg.end_timestamp_ms - seg.start_timestamp_ms;
            println!("2000ms segment {}: {:.0}ms duration", i, duration_ms);
            // Each segment should be at least 250ms (min_speech_time)
            assert!(
                duration_ms >= 200.0,
                "Segment {} too short: {:.0}ms",
                i,
                duration_ms
            );
        }
    }

    /// 手动诊断测试：对真实录音扫描不同 VAD 阈值配置（不进 CI）。
    /// 运行：cargo test --lib vad_real_recording_threshold_sweep -- --ignored --nocapture
    #[test]
    #[ignore]
    fn vad_real_recording_threshold_sweep() {
        let path = std::path::Path::new(
            r"C:\Users\huawei\recordings\录音_2026-07-18_20-47_2026-07-18_12-47\audio.mp4",
        );
        if !path.exists() {
            eprintln!("SKIP: {} not found", path.display());
            return;
        }
        let decoded = crate::audio::decoder::decode_audio_file(path).expect("decode failed");
        println!(
            "decoded: {:.1}s {}Hz {}ch",
            decoded.duration_seconds, decoded.sample_rate, decoded.channels
        );
        let mono = if decoded.channels > 1 {
            crate::audio::audio_processing::audio_to_mono(&decoded.samples, decoded.channels)
        } else {
            decoded.samples
        };
        // Same normalization as the retranscription pipeline
        let max_abs = mono
            .iter()
            .filter(|s| s.is_finite())
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        let mut mono = if max_abs > 1.0 {
            let scale = 1.0 / max_abs;
            mono.into_iter().map(|s| s * scale).collect()
        } else {
            mono
        };
        for s in &mut mono {
            if !s.is_finite() {
                *s = 0.0;
            } else {
                *s = s.clamp(-1.0, 1.0);
            }
        }
        let sample_rate = decoded.sample_rate;

        let configs = [
            ("current   (0.45/0.30/1500)", 0.45f32, 0.30f32, 1500u32),
            ("cand-A    (0.40/0.28/1000)", 0.40, 0.28, 1000),
            ("medium    (0.35/0.25/ 800)", 0.35, 0.25, 800),
            ("cand-B    (0.32/0.22/ 700)", 0.32, 0.22, 700),
            ("cand-C    (0.30/0.20/ 800)", 0.30, 0.20, 800),
            ("sensitive (0.25/0.15/ 600)", 0.25, 0.15, 600),
        ];
        for (name, pos, neg, red) in configs {
            let mut p =
                ContinuousVadProcessor::new_with_thresholds(sample_rate, red, pos, neg).unwrap();
            let mut segs = p.process_audio(&mono).unwrap();
            segs.extend(p.flush().unwrap());
            println!("\n== {} == {} segments", name, segs.len());
            for s in &segs {
                println!(
                    "  [{:8.2} -> {:8.2}] {:6.2}s, {} samples",
                    s.start_timestamp_ms / 1000.0,
                    s.end_timestamp_ms / 1000.0,
                    (s.end_timestamp_ms - s.start_timestamp_ms) / 1000.0,
                    s.samples.len()
                );
            }
        }
    }
}
