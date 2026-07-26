// audio/transcription/x_asr_provider.rs
//
// X-ASR streaming transcription provider (Rust-native via sherpa-onnx OnlineRecognizer).
//
// Sentence segmentation rules (designed for translation quality):
//   1. Punctuation-first with length accumulation: complete sentences ending
//      with 。！？.!? are buffered until the buffered text reaches
//      SENTENCE_COMMIT_UNITS (CJK chars + non-CJK words), then committed
//      together at the last sentence boundary. Short sentences such as
//      "Thank you." therefore merge into one line instead of one line each.
//   2. (Currently disabled) VAD boundary + punctuation: commit after a short
//      800 ms stability window when a VAD boundary arrives and the pending
//      text already ends with sentence punctuation.
//   3. (Currently disabled) VAD boundary without punctuation: commit only
//      after a long silence (6 s, or 10 s for grammatically incomplete
//      endings such as trailing prepositions/conjunctions).
//      NOTE: rules 2-3 are dead code today — boundary_pending is never set;
//      VAD boundaries are only used for segment timestamps.
//   4. Length fallback (punctuation-free speech): when no strong sentence
//      boundary matches and the uncommitted text reaches SOFT_COMMIT_UNITS,
//      commit at the last weak boundary (，、；： , ; space); at
//      HARD_COMMIT_UNITS with no usable weak boundary, cut hard there.
//   5. Pause commit: audio silent for >= 2 s (chunk RMS <= 0.01) with
//      >= MIN_SEGMENT_UNITS uncommitted text commits the pending text —
//      a speaker pause is a natural segment boundary.
//   6. Stall timeout: if the recognized text has stopped growing for 30 s
//      (silence or stalled recognition), force-commit the pending text.
//      The clock tracks text growth, not commits, so accumulating short
//      sentences are never cut off mid-speech.
//   7. Input finished: commit all remaining text when streaming ends.

use super::provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use crate::audio::AudioChunk;
use crate::sherpa_onnx_engine::XAsrOnlineEngine;
use async_trait::async_trait;
use log::{info, warn};
use std::any::Any;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::mpsc;

use crate::audio::vad::ContinuousVadProcessor;

/// Global sequence counter for X-ASR committed segments.
static XASR_SEQUENCE_COUNTER: AtomicU64 = AtomicU64::new(0);

const MIN_COMMIT_CHARS: usize = 10;
/// 句级提交的最小缓冲（单位：CJK 字数 + 非 CJK 词数）：达到该值即在
/// 第一处句界提交（一句一行，句级翻译流水线）；不足则继续积攒，
/// 避免 "Thank you. Oh." 这类碎片独占一行。
const MIN_SEGMENT_UNITS: usize = 8;
/// 长度兜底软阈值（单位口径同 find_commit_boundary）：无强句界且未提交
/// 文本达到该值时，在最后一个弱边界（，、；： , ; 空格）处提交。
const SOFT_COMMIT_UNITS: usize = 48;
/// 长度兜底硬阈值：达到该值仍无可用弱边界时，在该处硬切提交，
/// 防止无标点语流下缓冲区无限增长。
const HARD_COMMIT_UNITS: usize = 80;
/// Stability required before committing a non-punctuation VAD boundary.
const NON_PUNCT_BOUNDARY_STABLE_MS: u64 = 6000;
/// Extra-long stability for VAD boundaries that end with a clearly incomplete phrase.
const INCOMPLETE_ENDING_STABLE_MS: u64 = 10000;

pub fn reset_xasr_sequence_counter() {
    XASR_SEQUENCE_COUNTER.store(0, Ordering::SeqCst);
    info!("🔍 X-ASR sequence counter reset to 0");
}

#[derive(Debug, Clone)]
struct BoundarySignal {
    start_time_sec: f64,
    end_time_sec: f64,
}

struct StreamingState {
    current_partial: String,
    committed_len: usize,
    current_sequence_id: u64,
    boundary_pending: bool,
    last_uncommitted: String,
    stable_since: Option<Instant>,
    segment_start_time: f64,
    segment_end_time: f64,
    input_finished: bool,
    /// Last time the recognized text changed; drives the 30 s stall timeout.
    last_text_growth: Instant,
    /// Whether segment_start_time has been anchored by a VAD boundary for the
    /// current segment. Reset after every commit so only the first boundary
    /// of a new segment sets its start (accumulated segments span several).
    has_segment_start: bool,
    /// Current audio time in seconds, updated from incoming chunk timestamps.
    current_audio_time: f64,
    /// Audio-stream timestamp of the last chunk whose RMS energy indicated
    /// speech; drives the >=2 s pause commit.
    last_speech_audio_time: f64,
}

impl StreamingState {
    fn uncommitted(&self) -> &str {
        &self.current_partial[self.committed_len..]
    }
}

pub struct XAsrProvider {
    model_name: String,
    engine: Arc<XAsrOnlineEngine>,
}

impl XAsrProvider {
    pub fn new_with_engine(model_name: String, engine: Arc<XAsrOnlineEngine>) -> Self {
        Self { model_name, engine }
    }

    /// Transcribe a single audio file using the Rust-native OnlineRecognizer.
    /// Expects 16-bit PCM WAV (mono or stereo, any sample rate is resampled).
    pub async fn transcribe_file(&self, path: &std::path::Path) -> Result<String, String> {
        let file_bytes = tokio::fs::read(path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Parse minimal WAV header
        if file_bytes.len() < 44 {
            return Err("File too small to be a valid WAV".to_string());
        }
        let riff_marker = &file_bytes[0..4];
        if riff_marker != b"RIFF" {
            return Err("Not a valid WAV file".to_string());
        }
        let channels = u16::from_le_bytes([file_bytes[22], file_bytes[23]]);
        let sample_rate = u32::from_le_bytes([file_bytes[24], file_bytes[25], file_bytes[26], file_bytes[27]]) as i32;
        let bits_per_sample = u16::from_le_bytes([file_bytes[34], file_bytes[35]]);

        if bits_per_sample != 16 {
            return Err(format!("Only 16-bit WAV supported, got {} bits", bits_per_sample));
        }

        let data_offset = 44u32; // skip standard header
        let raw_samples: Vec<i16> = file_bytes[data_offset as usize..]
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();

        if raw_samples.is_empty() {
            return Err("Empty audio file".to_string());
        }

        let samples: Vec<f32> = if channels == 1 {
            raw_samples.iter().map(|&s| s as f32 / 32768.0).collect()
        } else {
            // Downmix stereo to mono
            raw_samples
                .chunks_exact(channels as usize)
                .map(|ch| {
                    let sum: f32 = ch.iter().map(|&s| s as f32).sum();
                    sum / (ch.len() as f32 * 32768.0)
                })
                .collect()
        };

        if samples.is_empty() {
            return Err("Empty audio file".to_string());
        }

        let stream = self.engine.recognizer.create_stream();

        // Add 500ms silence padding for context
        let pad_len = (sample_rate as f32 * 0.5) as usize;
        let silence = vec![0.0f32; pad_len];
        stream.accept_waveform(sample_rate, &silence);
        while self.engine.recognizer.is_ready(&stream) {
            self.engine.recognizer.decode(&stream);
        }

        // Feed all samples in chunks to avoid huge single accept_waveform calls
        for chunk in samples.chunks(sample_rate as usize) {
            stream.accept_waveform(sample_rate, chunk);
            while self.engine.recognizer.is_ready(&stream) {
                self.engine.recognizer.decode(&stream);
            }
        }

        // Post-padding
        stream.accept_waveform(sample_rate, &silence);
        while self.engine.recognizer.is_ready(&stream) {
            self.engine.recognizer.decode(&stream);
        }

        stream.input_finished();
        while self.engine.recognizer.is_ready(&stream) {
            self.engine.recognizer.decode(&stream);
        }

        match self.engine.recognizer.get_result(&stream) {
            Some(result) => Ok(result.text),
            None => Ok(String::new()),
        }
    }

    /// Run continuous streaming transcription with audio-level segmentation.
    pub async fn run_streaming<R: Runtime>(
        &self,
        mut receiver: tokio::sync::mpsc::UnboundedReceiver<AudioChunk>,
        app: AppHandle<R>,
    ) {
        info!("🎙️ X-ASR streaming starting (Rust-native), model={}", self.model_name);

        let stream = self.engine.recognizer.create_stream();
        let mut segmenter = match ContinuousVadProcessor::new(16000, 1500) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to create X-ASR segmenter: {}", e);
                return;
            }
        };

        let (boundary_tx, mut boundary_rx) = mpsc::unbounded_channel::<BoundarySignal>();

        let state = Arc::new(Mutex::new(StreamingState {
            current_partial: String::new(),
            committed_len: 0,
            current_sequence_id: XASR_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
            boundary_pending: false,
            last_uncommitted: String::new(),
            stable_since: None,
            segment_start_time: 0.0,
            segment_end_time: 0.0,
            input_finished: false,
            last_text_growth: Instant::now(),
            has_segment_start: false,
            current_audio_time: 0.0,
            last_speech_audio_time: 0.0,
        }));

        const STABLE_MS: u64 = 800; // must be stable 800ms before committing (avoids mid-word splits like "you'" / "re")
        const FINAL_WAIT_MS: u64 = 800;

        let mut input_finished = false;
        let mut final_wait_deadline: Option<Instant> = None;
        let mut boundary_open = true;
        let mut last_decode_time = Instant::now();
        let mut channel_closed = false;

        loop {
            let poll_duration = if input_finished {
                Duration::from_millis(50)
            } else {
                Duration::from_millis(10)
            };

            tokio::select! {
                biased;
                chunk = receiver.recv(), if !channel_closed => {
                    match chunk {
                        Some(chunk) => {
                            let samples_16k = if chunk.sample_rate != 16000 {
                                crate::audio::audio_processing::resample_audio(
                                    &chunk.data, chunk.sample_rate, 16000,
                                )
                            } else {
                                chunk.data
                            };

                            stream.accept_waveform(16000, &samples_16k);
                            while self.engine.recognizer.is_ready(&stream) {
                                self.engine.recognizer.decode(&stream);
                            }
                            last_decode_time = Instant::now();

                            {
                                let mut s = state.lock().unwrap();
                                s.current_audio_time = chunk.timestamp;
                                // RMS 能量检测有声段：> 0.01 视为有声，用于停顿提交
                                if !samples_16k.is_empty() {
                                    let rms = (samples_16k.iter().map(|x| x * x).sum::<f32>()
                                        / samples_16k.len() as f32)
                                        .sqrt();
                                    if rms > 0.01 {
                                        s.last_speech_audio_time = chunk.timestamp;
                                    }
                                }
                                drop(s);
                            }

                            if let Some(result) = self.engine.recognizer.get_result(&stream) {
                                if !result.text.is_empty() {
                                    Self::handle_partial(&state, &app, result.text.clone());
                                }
                            }

                            match segmenter.process_audio(&samples_16k) {
                                Ok(segments) => {
                                    for segment in segments {
                                        if segment.samples.len() >= 800 {
                                            let _ = boundary_tx.send(BoundarySignal {
                                                start_time_sec: segment.start_timestamp_ms / 1000.0,
                                                end_time_sec: segment.end_timestamp_ms / 1000.0,
                                            });
                                        }
                                    }
                                }
                                Err(e) => warn!("X-ASR segmenter error: {}", e),
                            }
                        }
                        None => {
                            channel_closed = true;
                            if !input_finished {
                                input_finished = true;
                                {
                                    let mut s = state.lock().unwrap();
                                    s.input_finished = true;
                                }
                                final_wait_deadline = Some(
                                    Instant::now() + Duration::from_millis(FINAL_WAIT_MS),
                                );

                                match segmenter.flush() {
                                    Ok(segments) => {
                                        for segment in segments {
                                            if segment.samples.len() >= 800 {
                                                let _ = boundary_tx.send(BoundarySignal {
                                                    start_time_sec: segment.start_timestamp_ms / 1000.0,
                                                    end_time_sec: segment.end_timestamp_ms / 1000.0,
                                                });
                                            }
                                        }
                                    }
                                    Err(e) => warn!("X-ASR segmenter flush error: {}", e),
                                }

                                stream.input_finished();
                                while self.engine.recognizer.is_ready(&stream) {
                                    self.engine.recognizer.decode(&stream);
                                }
                                if let Some(result) = self.engine.recognizer.get_result(&stream) {
                                    Self::handle_final(&state, &app, result.text);
                                }

                                info!("🎙️ X-ASR audio input finished, waiting for final segments");
                            }
                        }
                    }
                }
                boundary = boundary_rx.recv(), if boundary_open => {
                    if let Some(boundary) = boundary {
                        let mut s = state.lock().unwrap();
                        // The first VAD boundary after a commit anchors the new
                        // segment's start; later boundaries only extend its end.
                        // This keeps timestamps correct when one committed
                        // segment accumulates several utterances.
                        if !s.has_segment_start {
                            s.segment_start_time = boundary.start_time_sec;
                            s.has_segment_start = true;
                        }
                        s.segment_end_time = boundary.end_time_sec;
                    } else {
                        boundary_open = false;
                    }
                }
                _ = tokio::time::sleep(poll_duration) => {
                    // Poll recognizer — always poll after input finished to catch final results
                    let should_poll = input_finished || last_decode_time.elapsed() < Duration::from_secs(2);
                    if should_poll {
                        if let Some(result) = self.engine.recognizer.get_result(&stream) {
                            if !result.text.is_empty() {
                                Self::handle_partial(&state, &app, result.text);
                            }
                        }
                    }

                    // Commit complete sentences from uncommitted text (punctuation-based)
                    Self::try_commit_sentences(&state, &app);

                    // Stall timeout: force-commit if the recognized text has
                    // stopped growing for 30 s. The clock tracks text growth,
                    // so accumulating short sentences are not cut mid-speech.
                    {
                        let s = state.lock().unwrap();
                        let uncommitted = s.uncommitted().to_string();
                        let timed_out = s.last_text_growth.elapsed() > Duration::from_secs(30)
                            && !uncommitted.trim().is_empty();
                        drop(s);
                        if timed_out {
                            info!("⏰ X-ASR 30s stall timeout — force committing remaining text");
                            Self::force_commit_remaining(&state, &app);
                        }
                    }

                    // Pause commit: audio has been silent for >= 2 s while text
                    // is still uncommitted. A speaker pause is a natural segment
                    // boundary, so we commit even if the text was still growing
                    // recently (last_text_growth is close) — this is intentional.
                    {
                        let s = state.lock().unwrap();
                        let pause_commit = s.last_speech_audio_time > 0.0
                            && s.current_audio_time - s.last_speech_audio_time >= 2.0
                            && text_length_units(s.uncommitted()) >= MIN_SEGMENT_UNITS;
                        drop(s);
                        if pause_commit {
                            info!("⏸️ X-ASR pause commit (>=2s silence)");
                            Self::force_commit_remaining(&state, &app);
                        }
                    }

                    Self::try_commit_boundary(&state, &app, STABLE_MS);

                    if input_finished {
                        if let Some(deadline) = final_wait_deadline {
                            if Instant::now() >= deadline {
                                Self::force_commit_remaining(&state, &app);
                                break;
                            }
                        }
                    }
                }
            }
        }

        info!("🎙️ X-ASR streaming ended (Rust-native)");
    }

    fn handle_partial<R: Runtime>(state: &Arc<Mutex<StreamingState>>, app: &AppHandle<R>, text: String) {
        let mut s = state.lock().unwrap();
        s.current_partial = text;

        let uncommitted = s.uncommitted().to_string();
        if !uncommitted.is_empty() && s.last_uncommitted != uncommitted {
            s.last_uncommitted = uncommitted.clone();
            s.last_text_growth = Instant::now();
            let audio_end = s.current_audio_time;
            let audio_start = (audio_end - estimate_audio_duration(&uncommitted)).max(0.0);
            let update = super::worker::TranscriptUpdate {
                text: uncommitted,
                timestamp: format_timestamp_simple(),
                source: "Audio".to_string(),
                sequence_id: s.current_sequence_id,
                chunk_start_time: 0.0,
                is_partial: true,
                confidence: 0.9,
                audio_start_time: audio_start,
                audio_end_time: audio_end,
                duration: audio_end - audio_start,
            };
            if let Err(e) = app.emit("transcript-update", &update) {
                warn!("X-ASR emit partial failed: {}", e);
            }
        }
    }

    fn handle_final<R: Runtime>(state: &Arc<Mutex<StreamingState>>, app: &AppHandle<R>, text: String) {
        let mut s = state.lock().unwrap();
        s.current_partial = text;

        let uncommitted = s.uncommitted().to_string();
        if !uncommitted.is_empty() {
            Self::commit_segment(&mut s, app, uncommitted);
        }
    }

    fn try_commit_boundary<R: Runtime>(
        state: &Arc<Mutex<StreamingState>>,
        app: &AppHandle<R>,
        stable_ms: u64,
    ) -> bool {
        let mut s = state.lock().unwrap();
        if !s.boundary_pending {
            return false;
        }

        let uncommitted = s.uncommitted().to_string();

        if uncommitted != s.last_uncommitted {
            s.last_uncommitted = uncommitted;
            s.stable_since = Some(Instant::now());
            return false;
        }

        // Decide how long the text must be stable before we are willing to commit
        // a VAD boundary. The goal is to avoid splitting mid-sentence pauses such as
        // "some of you ... are probably wishing ...".
        let (sentence_end, has_sentence_boundary) = Self::find_last_sentence_boundary(&uncommitted);
        let ends_with_punctuation = has_sentence_boundary && sentence_end >= uncommitted.len();
        let required_stable_ms = if ends_with_punctuation || s.input_finished {
            stable_ms
        } else if Self::is_likely_incomplete_ending(&uncommitted) {
            // The text ends with a word/phrase that strongly suggests the speaker
            // will continue (e.g. "and", "to", "some of you"). Wait extra long.
            INCOMPLETE_ENDING_STABLE_MS
        } else {
            // Non-punctuation boundary with a neutral ending: only commit after a
            // long silence, which usually means the speaker really paused between
            // utterances.
            NON_PUNCT_BOUNDARY_STABLE_MS
        };

        if let Some(since) = s.stable_since {
            if since.elapsed() < Duration::from_millis(required_stable_ms) {
                return false;
            }
        }

        if uncommitted.chars().count() >= MIN_COMMIT_CHARS || s.input_finished {
            s.boundary_pending = false;
            s.stable_since = None;
            let text = uncommitted.clone();
            let committed_len_before = s.committed_len;
            s.committed_len += text.len();
            drop(s);
            Self::commit_segment_clone(state, app, text, committed_len_before);
            return true;
        }

        false
    }

    /// Heuristic check for phrases that should not be committed at a VAD boundary
    /// because they are almost certainly the middle of a sentence.
    fn is_likely_incomplete_ending(text: &str) -> bool {
        let trimmed = text
            .trim_end_matches(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | ':' | '.' | '?' | '!'))
            .to_lowercase();
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.is_empty() {
            return false;
        }
        let last = *tokens.last().unwrap();
        let prev2 = tokens.len().checked_sub(2).map(|i| &tokens[i..]);

        // Words that rarely end a sentence in English / Chinese-English mixed speech.
        const INCOMPLETE_WORDS: &[&str] = &[
            // conjunctions
            "and", "but", "or", "nor", "so", "yet", "because", "since", "if", "when",
            "where", "while", "although", "though", "unless", "whether", "before", "after",
            // prepositions
            "in", "on", "at", "to", "for", "of", "from", "with", "by", "about", "into",
            "through", "during", "above", "below", "under", "between", "among", "around",
            // relative / demonstrative / question words
            "who", "whom", "whose", "which", "that", "what", "this", "these", "those",
            // quantifiers / pronouns that usually need a noun/verb after them
            "some", "many", "most", "all", "none", "any", "each", "every", "both", "either",
            "neither", "few", "several",
            // auxiliary / modal verbs
            "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do",
            "does", "did", "can", "could", "will", "would", "shall", "should", "may", "might",
            "must",
            // determiners / possessives
            "a", "an", "the", "my", "your", "his", "her", "its", "our", "their",
        ];

        if INCOMPLETE_WORDS.contains(&last) {
            return true;
        }

        // Common two-word tails that almost always continue.
        if let Some(&[a, b]) = prev2 {
            match (a, b) {
                ("some", "of") | ("many", "of") | ("most", "of") | ("all", "of")
                | ("none", "of") | ("each", "of") | ("some", "or") | ("more", "or")
                | ("one", "more") | ("just", "a") | ("just", "one") | ("a", "lot")
                | ("a", "few") | ("sort", "of") | ("kind", "of") | ("out", "there")
                | ("up", "to") | ("due", "to") | ("thanks", "to") => return true,
                _ => {}
            }
        }

        // Trailing hyphen usually means a word is being split across pauses.
        if last.ends_with('-') {
            return true;
        }

        false
    }

    /// Find the byte position after the LAST complete sentence boundary.
    /// A complete sentence ends with 。！？.!? followed by whitespace, line break, or CJK char.
    /// Returns (byte_offset, found) where byte_offset is safe to use with string slicing.
    fn find_last_sentence_boundary(text: &str) -> (usize, bool) {
        let char_indices: Vec<(usize, char)> = text.char_indices().collect();
        let len = char_indices.len();
        for i in (0..len).rev() {
            let (_, c) = char_indices[i];
            let is_sentence_end = matches!(c, '。' | '！' | '？' | '.' | '!' | '?');
            if !is_sentence_end {
                continue;
            }
            if i + 1 >= len {
                return (text.len(), true);
            }
            let (_, next) = char_indices[i + 1];
            if matches!(next, ' ' | '\n' | '\r' | '"' | '\u{300D}' | '\u{FF09}' | '\u{3011}' | '」' | '）' | '】')
                || next as u32 >= 0x4e00
            {
                let mut end_idx = i + 1;
                while end_idx < len && matches!(char_indices[end_idx].1, ' ' | '\n' | '\r') {
                    end_idx += 1;
                }
                if end_idx < len {
                    return (char_indices[end_idx].0, true);
                } else {
                    return (text.len(), true);
                }
            }
        }
        (0, false)
    }

    /// Find the FIRST sentence boundary whose prefix has at least `min_units`
    /// text units (CJK chars + non-CJK words). Returns byte offset (safe for
    /// slicing) or None. This yields sentence-per-line commits for normal
    /// sentences while still grouping tiny fragments like "Thank you. Oh."
    fn find_commit_boundary(text: &str, min_units: usize) -> Option<usize> {
        let char_indices: Vec<(usize, char)> = text.char_indices().collect();
        let len = char_indices.len();
        let mut units = 0usize;
        let mut in_word = false;

        for (i, (_, c)) in char_indices.iter().enumerate() {
            // text_length_units 同款口径：CJK 每字 1，非 CJK 每个词 1
            if (*c as u32) >= 0x4E00 {
                units += 1;
                in_word = false;
            } else if c.is_whitespace() {
                in_word = false;
            } else if !in_word {
                units += 1;
                in_word = true;
            }

            let is_sentence_end = matches!(c, '。' | '！' | '？' | '.' | '!' | '?');
            if !is_sentence_end {
                continue;
            }
            let next = char_indices.get(i + 1).map(|(_, c)| *c);
            let boundary_ok = match next {
                None => true,
                Some(n) => {
                    n.is_whitespace()
                        || matches!(n, '"' | '\u{300D}' | '\u{FF09}' | '\u{3011}' | '」' | '）' | '】')
                        || (n as u32) >= 0x4e00
                }
            };
            if !boundary_ok {
                continue;
            }
            // 跳过标点后的空白，得到切分点
            let mut end_idx = i + 1;
            while end_idx < len && matches!(char_indices[end_idx].1, ' ' | '\n' | '\r') {
                end_idx += 1;
            }
            let boundary = if end_idx < len {
                char_indices[end_idx].0
            } else {
                text.len()
            };
            if units >= min_units {
                return Some(boundary);
            }
        }
        None
    }

    /// Length-based fallback boundary for punctuation-free speech. Uses the
    /// same unit metric as find_commit_boundary (CJK chars + non-CJK words):
    /// once the uncommitted text reaches `soft` units, commit at the LAST weak
    /// boundary (，、；： , ; or whitespace) whose prefix has at least `min`
    /// units; if it reaches `hard` units with no usable weak boundary, cut
    /// hard at the `hard`-unit position. Returns byte offset or None.
    fn find_length_boundary(
        text: &str,
        soft: usize,
        hard: usize,
        min: usize,
    ) -> Option<usize> {
        let char_indices: Vec<(usize, char)> = text.char_indices().collect();
        let len = char_indices.len();
        let mut units = 0usize;
        let mut in_word = false;
        // (byte offset after the weak boundary, units counted up to it)
        let mut last_weak: Option<(usize, usize)> = None;

        for (i, (byte_idx, c)) in char_indices.iter().enumerate() {
            if (*c as u32) >= 0x4E00 {
                units += 1;
                in_word = false;
            } else if c.is_whitespace() {
                in_word = false;
            } else if !in_word {
                units += 1;
                in_word = true;
            }

            let is_weak = matches!(c, '，' | '、' | '；' | '：' | ',' | ';' | ' ');
            if is_weak {
                // 跳过边界符后的空白，得到切分点
                let mut end_idx = i + 1;
                while end_idx < len && char_indices[end_idx].1.is_whitespace() {
                    end_idx += 1;
                }
                let boundary = if end_idx < len {
                    char_indices[end_idx].0
                } else {
                    text.len()
                };
                last_weak = Some((boundary, units));
            }

            if units >= hard {
                // 硬阈值：优先用最后一个满足最小单元的弱边界，否则硬切
                if let Some((boundary, weak_units)) = last_weak {
                    if weak_units >= min {
                        return Some(boundary);
                    }
                }
                return Some(byte_idx + c.len_utf8());
            }
        }

        if units >= soft {
            if let Some((boundary, weak_units)) = last_weak {
                if weak_units >= min {
                    return Some(boundary);
                }
            }
        }
        None
    }

    /// Commit the first complete sentence (or minimal sentence group) from the
    /// uncommitted text. Sentence-per-line pipeline: each commit is translated
    /// individually — OPUS-MT is a sentence-level model and does best with
    /// sentence-sized input; long multi-sentence segments trigger dropping.
    fn try_commit_sentences<R: Runtime>(
        state: &Arc<Mutex<StreamingState>>,
        app: &AppHandle<R>,
    ) -> bool {
        let mut s = state.lock().unwrap();
        let uncommitted = s.uncommitted().to_string();
        // 强句界优先；无强句界时走长度兜底（弱边界软切 / 超硬阈值硬切），
        // 防止无标点语流下缓冲区无限增长。
        let Some(sentence_end) = Self::find_commit_boundary(&uncommitted, MIN_SEGMENT_UNITS)
            .or_else(|| {
                Self::find_length_boundary(
                    &uncommitted,
                    SOFT_COMMIT_UNITS,
                    HARD_COMMIT_UNITS,
                    MIN_SEGMENT_UNITS,
                )
            })
        else {
            return false;
        };

        let sentence = uncommitted[..sentence_end].trim().to_string();
        if sentence.chars().filter(|c| !c.is_whitespace()).count() < 2 {
            return false;
        }

        let committed_len_before = s.committed_len;
        s.committed_len += sentence_end;
        s.last_text_growth = Instant::now();
        drop(s);
        Self::commit_segment_clone(state, app, sentence, committed_len_before);
        true
    }

    fn commit_segment_clone<R: Runtime>(
        state: &Arc<Mutex<StreamingState>>,
        app: &AppHandle<R>,
        text: String,
        committed_len_before: usize,
    ) {
        let s = state.lock().unwrap();
        let seg_id = s.current_sequence_id;
        let audio_end = s.current_audio_time.max(s.segment_end_time);
        let audio_start = s.segment_start_time.max(0.0);
        drop(s);

        if text.is_empty() {
            return;
        }

        // Fall back to a duration estimate if we do not have reliable boundary times.
        let (final_start, final_end) = if audio_start > 0.0 && audio_end > audio_start {
            (audio_start, audio_end)
        } else {
            let end = audio_end.max(0.0);
            let start = (end - estimate_audio_duration(&text)).max(0.0);
            (start, end)
        };

        let update = super::worker::TranscriptUpdate {
            text: text.clone(),
            timestamp: format_timestamp_simple(),
            source: "Audio".to_string(),
            sequence_id: seg_id,
            chunk_start_time: 0.0,
            is_partial: false,
            confidence: 0.9,
            audio_start_time: final_start,
            audio_end_time: final_end,
            duration: final_end - final_start,
        };
        if let Err(e) = app.emit("transcript-update", &update) {
            warn!("X-ASR emit commit failed: {}", e);
        }
        crate::translation::queue_translation(app, &update.text, update.sequence_id);

        // Advance to next segment
        let mut s = state.lock().unwrap();
        s.current_sequence_id = XASR_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst);
        if s.committed_len == committed_len_before + update.text.len() {
            s.committed_len = s.current_partial.len();
        }
        // Update the next segment start time so it does not reuse the old boundary.
        s.segment_start_time = final_end;
        s.segment_end_time = final_end;
        // Next VAD boundary anchors the new segment's start time.
        s.has_segment_start = false;
        info!(
            "✅ X-ASR committed seg seq={} chars={}",
            seg_id,
            update.text.len()
        );
    }

    fn commit_segment<R: Runtime>(
        s: &mut StreamingState,
        app: &AppHandle<R>,
        text: String,
    ) {
        if text.is_empty() {
            return;
        }
        let audio_end = s.current_audio_time.max(s.segment_end_time);
        let audio_start = s.segment_start_time.max(0.0);
        let (final_start, final_end) = if audio_start > 0.0 && audio_end > audio_start {
            (audio_start, audio_end)
        } else {
            let end = audio_end.max(0.0);
            let start = (end - estimate_audio_duration(&text)).max(0.0);
            (start, end)
        };
        let update = super::worker::TranscriptUpdate {
            text,
            timestamp: format_timestamp_simple(),
            source: "Audio".to_string(),
            sequence_id: s.current_sequence_id,
            chunk_start_time: 0.0,
            is_partial: false,
            confidence: 0.9,
            audio_start_time: final_start,
            audio_end_time: final_end,
            duration: final_end - final_start,
        };
        s.committed_len = s.current_partial.len();
        s.current_sequence_id = XASR_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst);
        s.segment_start_time = final_end;
        s.segment_end_time = final_end;
        // Next VAD boundary anchors the new segment's start time.
        s.has_segment_start = false;
        if let Err(e) = app.emit("transcript-update", &update) {
            warn!("X-ASR emit commit failed: {}", e);
        }
        crate::translation::queue_translation(app, &update.text, update.sequence_id);
    }

    fn force_commit_remaining<R: Runtime>(
        state: &Arc<Mutex<StreamingState>>,
        app: &AppHandle<R>,
    ) {
        let mut s = state.lock().unwrap();
        let uncommitted = s.uncommitted().to_string();
        if !uncommitted.is_empty() {
            s.boundary_pending = false;
            s.stable_since = None;
            s.last_text_growth = Instant::now();
            let text = uncommitted;
            let committed_len_before = s.committed_len;
            s.committed_len += text.len();
            drop(s);
            Self::commit_segment_clone(state, app, text, committed_len_before);
        } else {
            s.boundary_pending = false;
            s.stable_since = None;
        }
    }
}

#[async_trait]
impl TranscriptionProvider for XAsrProvider {
    async fn transcribe(
        &self,
        _audio: Vec<f32>,
        _language: Option<String>,
    ) -> Result<TranscriptResult, TranscriptionError> {
        Err(TranscriptionError::EngineFailed(
            "X-ASR uses streaming mode, not chunk-based transcribe()".into(),
        ))
    }

    async fn is_model_loaded(&self) -> bool {
        true
    }

    async fn get_current_model(&self) -> Option<String> {
        Some(self.model_name.clone())
    }

    fn provider_name(&self) -> &'static str {
        "x-asr"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn format_timestamp_simple() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let hours = (now.as_secs() / 3600) % 24;
    let minutes = (now.as_secs() / 60) % 60;
    let seconds = now.as_secs() % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Estimate how many seconds of audio the given text likely represents.
/// Heuristic: ~0.28s per CJK character and ~0.35s per non-CJK word.
fn estimate_audio_duration(text: &str) -> f64 {
    let cjk_count = text.chars().filter(|c| is_cjk(*c)).count() as f64;
    let non_cjk_word_count = text
        .split_whitespace()
        .flat_map(|w| w.split(|c: char| c.is_ascii_punctuation()))
        .filter(|w| !w.is_empty())
        .count() as f64;
    (cjk_count * 0.28 + non_cjk_word_count * 0.35).max(0.5)
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x4E00..=0x9FFF |
        0x3400..=0x4DBF |
        0xF900..=0xFAFF |
        0x20000..=0x2A6DF |
        0x2A700..=0x2B73F |
        0x2B740..=0x2B81F |
        0x2B820..=0x2CEAF |
        0x2F800..=0x2FA1F
    )
}

/// Text length in "units": 1 per CJK character plus 1 per non-CJK word.
/// Same metric as estimate_audio_duration, so a single SENTENCE_COMMIT_UNITS
/// threshold means roughly the same spoken length for Chinese and English.
fn text_length_units(text: &str) -> usize {
    let cjk = text.chars().filter(|c| is_cjk(*c)).count();
    let words = text
        .split_whitespace()
        .flat_map(|w| w.split(|c: char| c.is_ascii_punctuation()))
        .filter(|w| {
            !w.is_empty()
                && !w.chars().any(is_cjk)
                && w.chars().any(|c| c.is_alphanumeric())
        })
        .count();
    cjk + words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_count_english_words() {
        // "Thank you. Oh. Thank you." -> 5 words, punctuation ignored.
        assert_eq!(text_length_units("Thank you. Oh. Thank you."), 5);
        assert_eq!(text_length_units("Well, thank you so much."), 5);
    }

    #[test]
    fn units_count_cjk_chars() {
        // CJK ideographs count one each; Chinese punctuation does not count.
        assert_eq!(text_length_units("你好世界。"), 4);
        assert_eq!(text_length_units("谢谢，谢谢大家！"), 6);
    }

    #[test]
    fn units_mixed_text_counts_both() {
        // 3 CJK chars + 2 English words.
        assert_eq!(text_length_units("谢谢你 thank you"), 5);
    }

    #[test]
    fn units_ignore_empty_and_punct_only() {
        assert_eq!(text_length_units(""), 0);
        assert_eq!(text_length_units("..."), 0);
        assert_eq!(text_length_units("。！？"), 0);
    }

    #[test]
    fn commit_boundary_groups_tiny_sentences() {
        // 短句聚合到第一处达到最小单元的句界
        let text = "Thank you. Oh. Thank you. I love you back. Well, first of all.";
        let b = XAsrProvider::find_commit_boundary(text, MIN_SEGMENT_UNITS)
            .expect("should find boundary");
        assert_eq!(text[..b].trim(), "Thank you. Oh. Thank you. I love you back.");
    }

    #[test]
    fn commit_boundary_single_long_sentence() {
        // 长句在第一处句界单独成段
        let text = "I am still fired up and ready to go. Thank you. Thank you.";
        let b = XAsrProvider::find_commit_boundary(text, MIN_SEGMENT_UNITS)
            .expect("should find boundary");
        assert_eq!(text[..b].trim(), "I am still fired up and ready to go.");
    }

    #[test]
    fn commit_boundary_none_when_too_short() {
        assert!(XAsrProvider::find_commit_boundary("Thank you. ", MIN_SEGMENT_UNITS).is_none());
    }

    #[test]
    fn length_boundary_soft_cut_at_last_weak_boundary_cjk() {
        // 无强句界长文本（61 单位），在最后的弱边界 '，' 处软切
        let text = format!("{}，{}", "今".repeat(30), "明".repeat(30));
        let b = XAsrProvider::find_length_boundary(
            &text, SOFT_COMMIT_UNITS, HARD_COMMIT_UNITS, MIN_SEGMENT_UNITS,
        )
        .expect("should soft-cut at weak boundary");
        assert_eq!(text[..b].chars().count(), 31); // 30 字 + '，'
    }

    #[test]
    fn length_boundary_soft_cut_at_last_space_english() {
        // 60 个无标点英文词：软切在最后一个空格（前 59 词）
        let words: Vec<String> = (0..60).map(|i| format!("w{}", i)).collect();
        let text = words.join(" ");
        let b = XAsrProvider::find_length_boundary(
            &text, SOFT_COMMIT_UNITS, HARD_COMMIT_UNITS, MIN_SEGMENT_UNITS,
        )
        .expect("should soft-cut at last space");
        // 切分点在最后一词之前（含其前的分隔空格，提交侧会 trim）
        assert_eq!(text[..b].trim_end(), words[..59].join(" "));
    }

    #[test]
    fn length_boundary_hard_cut_without_weak_boundary() {
        // 90 个连续汉字、无任何弱边界：在 HARD_COMMIT_UNITS 处硬切
        let text = "汉".repeat(90);
        let b = XAsrProvider::find_length_boundary(
            &text, SOFT_COMMIT_UNITS, HARD_COMMIT_UNITS, MIN_SEGMENT_UNITS,
        )
        .expect("should hard-cut at HARD_COMMIT_UNITS");
        assert_eq!(text[..b].chars().count(), HARD_COMMIT_UNITS);
    }

    #[test]
    fn length_boundary_none_when_short() {
        // 低于软阈值不切
        let text = "汉".repeat(20);
        assert!(XAsrProvider::find_length_boundary(
            &text, SOFT_COMMIT_UNITS, HARD_COMMIT_UNITS, MIN_SEGMENT_UNITS,
        )
        .is_none());
    }

    #[test]
    fn length_boundary_none_when_weak_prefix_below_min() {
        // 弱边界前缀（6 单位）不足 MIN_SEGMENT_UNITS，且未达硬阈值：不切
        let text = format!("{}，{}", "今".repeat(5), "明".repeat(50));
        assert!(XAsrProvider::find_length_boundary(
            &text, SOFT_COMMIT_UNITS, HARD_COMMIT_UNITS, MIN_SEGMENT_UNITS,
        )
        .is_none());
    }

    #[test]
    fn length_boundary_hard_cut_when_weak_prefix_below_min() {
        // 弱边界前缀不足 min，但文本达到硬阈值：在硬阈值处硬切
        let text = format!("{}，{}", "今".repeat(5), "明".repeat(90));
        let b = XAsrProvider::find_length_boundary(
            &text, SOFT_COMMIT_UNITS, HARD_COMMIT_UNITS, MIN_SEGMENT_UNITS,
        )
        .expect("should hard-cut when weak prefix below min");
        assert_eq!(text[..b].chars().count(), HARD_COMMIT_UNITS);
    }
}
