use crate::api::TranscriptSegment;
use anyhow::Result;
use log::{debug, info};
use std::path::Path;
use uuid::Uuid;

/// Unload the transcription engine after a batch job (import or retranscription).
/// Skips unloading if a live recording is currently in progress.
pub(crate) async fn unload_engine_after_batch() {
    if crate::audio::recording_commands::is_recording().await {
        log::info!("Skipping model unload after batch: recording in progress");
        return;
    }
    // Sherpa-ONNX engine stays loaded for performance
    log::info!("Batch job complete - Sherpa-ONNX engine remains loaded");
}

/// 离线转写的一条结果（文本 + 毫秒时间戳 + 说话人编号）。
pub(crate) struct TranscriptEntry {
    pub text: String,
    pub start_ms: f64,
    pub end_ms: f64,
    pub speaker: String,
}

/// 单条上游分句超过该字数就按句末标点再切（2026-09-24）。
///
/// 为什么需要：离线批量识别的分句粒度**完全由上游决定**，而上游是按**停顿**切分句的。
/// Deepgram 批量实测：一段 39 秒连续说话（电影访谈，中间没有 >0.8s 的停顿）被切成
/// **一条 664 字**的 utterance（内部其实有十几句话），落库后就是一条巨型段落
/// （`transcripts_offline.json` 段#1，同一段音频实时链路只有 ~100 字段落）。
/// 上游分句不可控，所以在落段前按句末标点兜底切一次，让离线结果与实时结果体量一致
/// （实时链路的活跃单元上限是 12/20 秒，实测段落 ≤ ~250 字）。
const OFFLINE_MAX_ENTRY_CHARS: usize = 200;

/// 按句末标点把过长的分句切成多段（时间戳按字数比例分摊）。
///
/// 只在「超过阈值」时才动，且不改变文本内容（切分处保留标点），说话人沿用原值。
fn split_long_entry(e: &TranscriptEntry) -> Vec<TranscriptEntry> {
    let text = e.text.trim();
    let chars = text.chars().count();
    if chars <= OFFLINE_MAX_ENTRY_CHARS {
        return vec![TranscriptEntry {
            text: text.to_string(),
            start_ms: e.start_ms,
            end_ms: e.end_ms,
            speaker: e.speaker.clone(),
        }];
    }

    // 按句末标点切成「句子」（标点跟在前句尾部）
    let mut sentences: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '?' | '!' | '。' | '？' | '！' | '…' | ';' | '；') {
            let t = cur.trim();
            if !t.is_empty() {
                sentences.push(t.to_string());
            }
            cur.clear();
        }
    }
    if !cur.trim().is_empty() {
        sentences.push(cur.trim().to_string());
    }
    if sentences.len() <= 1 {
        // 整段没有句末标点：切不动就不切（宁可不切也不要切断词）
        return vec![TranscriptEntry {
            text: text.to_string(),
            start_ms: e.start_ms,
            end_ms: e.end_ms,
            speaker: e.speaker.clone(),
        }];
    }

    // 贪心合并到接近阈值；单句超阈值就单独成段
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut buf: Vec<String> = Vec::new();
    let mut buf_chars = 0usize;
    for s in sentences {
        let n = s.chars().count();
        if !buf.is_empty() && buf_chars + n > OFFLINE_MAX_ENTRY_CHARS {
            groups.push(std::mem::take(&mut buf));
            buf_chars = 0;
        }
        buf.push(s);
        buf_chars += n;
    }
    if !buf.is_empty() {
        groups.push(buf);
    }

    // 时间戳按累计字数比例分摊（近似对齐；播放跳转够用）
    let span = (e.end_ms - e.start_ms).max(0.0);
    let mut out = Vec::with_capacity(groups.len());
    let mut consumed = 0usize;
    for g in groups {
        let g_text = g.join(" ");
        let g_chars = g_text.chars().count();
        let start = e.start_ms + span * (consumed as f64 / chars as f64);
        consumed += g_chars;
        let end = e.start_ms + span * (consumed as f64 / chars as f64);
        out.push(TranscriptEntry {
            text: g_text,
            start_ms: start,
            end_ms: end.max(start),
            speaker: e.speaker.clone(),
        });
        // 合并时插入的空格不计入原字数，累计作微调
        consumed += g.len().saturating_sub(g_chars);
        out.last_mut().unwrap().end_ms = e.start_ms + span * ((consumed.min(chars)) as f64 / chars as f64);
    }
    out
}

/// Create transcript segments from transcription results.
/// Each entry is (text, start_ms, end_ms, speaker) from VAD timestamps（说话人可为空串）。
/// 过长的上游分句会先按句末标点兜底切分（见 `split_long_entry`）。
pub(crate) fn create_transcript_segments(
    transcripts: &[TranscriptEntry],
) -> Vec<TranscriptSegment> {
    transcripts
        .iter()
        .flat_map(|e| split_long_entry(e))
        .map(|e| {
            let start_seconds = e.start_ms / 1000.0;
            let end_seconds = e.end_ms / 1000.0;
            let duration = end_seconds - start_seconds;

            TranscriptSegment {
                id: format!("transcript-{}", Uuid::new_v4()),
                text: e.text.trim().to_string(),
                timestamp: Some(chrono::Utc::now().to_rfc3339()),
                display_time: None,
                audio_start_time: Some(start_seconds),
                audio_end_time: Some(end_seconds),
                duration: Some(duration),
                speaker: if e.speaker.is_empty() {
                    None
                } else {
                    Some(e.speaker.clone())
                },
                translation: None,
            }
        })
        .collect()
}

/// Write transcripts.json to a meeting folder (atomic write with temp file)
pub(crate) fn write_transcripts_json(folder: &Path, segments: &[TranscriptSegment]) -> Result<()> {
    let transcript_path = folder.join("transcripts.json");
    let temp_path = folder.join(".transcripts.json.tmp");

    let json = serde_json::json!({
        "version": "1.0",
        "last_updated": chrono::Utc::now().to_rfc3339(),
        "total_segments": segments.len(),
        "segments": segments.iter().enumerate().map(|(i, s)| {
            serde_json::json!({
                "id": s.id,
                "text": s.text,
                "timestamp": s.timestamp,
                "audio_start_time": s.audio_start_time,
                "audio_end_time": s.audio_end_time,
                "duration": s.duration,
                "sequence_id": i,
                "translation": s.translation
            })
        }).collect::<Vec<_>>()
    });

    let json_string = serde_json::to_string_pretty(&json)?;
    std::fs::write(&temp_path, &json_string)?;
    std::fs::rename(&temp_path, &transcript_path)?;

    info!(
        "Wrote transcripts.json with {} segments to {}",
        segments.len(),
        transcript_path.display()
    );
    Ok(())
}

/// Split a long speech segment at the lowest-energy (silence) point near the target size.
///
/// Scans for 100ms windows with minimal RMS energy within +/-3 seconds of each target
/// split point. If no clear silence is found, falls back to a 1-second overlap split
/// to avoid cutting words at boundaries.
pub(crate) fn split_segment_at_silence(
    segment: &crate::audio::vad::SpeechSegment,
    max_samples: usize,
) -> Vec<crate::audio::vad::SpeechSegment> {
    const SAMPLE_RATE: usize = 16000;
    // 100ms window for energy measurement (1600 samples at 16kHz)
    const ENERGY_WINDOW: usize = SAMPLE_RATE / 10;
    // Search +/-3 seconds around the target split point
    const SEARCH_RADIUS: usize = SAMPLE_RATE * 3;
    // RMS threshold below which we consider a window "silent"
    const SILENCE_RMS_THRESHOLD: f32 = 0.02;
    // Overlap to use when no silence boundary is found (1 second)
    const FALLBACK_OVERLAP: usize = SAMPLE_RATE;

    let total = segment.samples.len();
    if total <= max_samples {
        return vec![segment.clone()];
    }

    let ms_per_sample =
        (segment.end_timestamp_ms - segment.start_timestamp_ms) / segment.samples.len() as f64;
    let mut result = Vec::new();
    let mut pos = 0usize;

    while pos < total {
        let remaining = total - pos;
        if remaining <= max_samples {
            let chunk_samples = segment.samples[pos..].to_vec();
            let chunk_start_ms = segment.start_timestamp_ms + (pos as f64 * ms_per_sample);
            let chunk_end_ms = segment.end_timestamp_ms;
            result.push(crate::audio::vad::SpeechSegment {
                samples: chunk_samples,
                start_timestamp_ms: chunk_start_ms,
                end_timestamp_ms: chunk_end_ms,
                confidence: segment.confidence,
            });
            break;
        }

        let target = pos + max_samples;

        let search_start = target.saturating_sub(SEARCH_RADIUS).max(pos + SAMPLE_RATE);
        let search_end = (target + SEARCH_RADIUS).min(total.saturating_sub(ENERGY_WINDOW));

        let mut best_split = target.min(total);
        let mut best_rms = f32::MAX;

        if search_start + ENERGY_WINDOW <= search_end {
            let mut idx = search_start;
            while idx + ENERGY_WINDOW <= search_end {
                let window = &segment.samples[idx..idx + ENERGY_WINDOW];
                let rms = (window.iter().map(|s| s * s).sum::<f32>() / ENERGY_WINDOW as f32).sqrt();
                if rms < best_rms {
                    best_rms = rms;
                    best_split = idx + ENERGY_WINDOW / 2;
                }
                idx += SAMPLE_RATE / 100;
            }
        }

        let split_at = best_split;
        if best_rms <= SILENCE_RMS_THRESHOLD {
            debug!(
                "Splitting at silence boundary: sample {} (RMS={:.4})",
                split_at, best_rms
            );
        } else {
            debug!(
                "No silence found near target (best RMS={:.4}), splitting with overlap at sample {}",
                best_rms, split_at
            );
        }

        let chunk_end = if best_rms > SILENCE_RMS_THRESHOLD {
            (split_at + FALLBACK_OVERLAP).min(total)
        } else {
            split_at
        };

        let chunk_samples = segment.samples[pos..chunk_end].to_vec();
        let chunk_start_ms = segment.start_timestamp_ms + (pos as f64 * ms_per_sample);
        let chunk_end_ms = segment.start_timestamp_ms + (chunk_end as f64 * ms_per_sample);

        result.push(crate::audio::vad::SpeechSegment {
            samples: chunk_samples,
            start_timestamp_ms: chunk_start_ms,
            end_timestamp_ms: chunk_end_ms,
            confidence: segment.confidence,
        });

        pos = chunk_end;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{split_long_entry, split_segment_at_silence, TranscriptEntry, OFFLINE_MAX_ENTRY_CHARS};
    use crate::audio::vad::SpeechSegment;

    /// Over-long segments (real-time SenseVoice path) must be split into parts
    /// no longer than the limit, with continuous timestamps covering the
    /// original range.
    #[test]
    fn split_long_segment_respects_max_and_keeps_timestamps_continuous() {
        const SR: usize = 16000;
        let max_samples = 15 * SR;
        // 40s of loud speech with clear 300ms silence gaps at 13s and 27s,
        // placed before the 15s split targets so parts stay under the limit.
        let total = 40 * SR;
        let mut samples = vec![0.5f32; total];
        for gap_start in [13 * SR, 27 * SR] {
            for s in &mut samples[gap_start..gap_start + SR * 3 / 10] {
                *s = 0.0;
            }
        }

        let segment = SpeechSegment {
            samples,
            start_timestamp_ms: 1000.0,
            end_timestamp_ms: 41000.0,
            confidence: 1.0,
        };

        let parts = split_segment_at_silence(&segment, max_samples);
        assert!(
            parts.len() >= 2,
            "40s segment should be split, got {} part(s)",
            parts.len()
        );
        for part in &parts {
            assert!(
                part.samples.len() <= max_samples,
                "part exceeds limit: {} > {} samples",
                part.samples.len(),
                max_samples
            );
        }
        // First part starts at the segment start, last ends at the segment end,
        // and part boundaries are contiguous.
        assert_eq!(parts.first().unwrap().start_timestamp_ms, 1000.0);
        assert_eq!(parts.last().unwrap().end_timestamp_ms, 41000.0);
        for w in parts.windows(2) {
            assert!(
                (w[0].end_timestamp_ms - w[1].start_timestamp_ms).abs() < 1e-6,
                "timestamp gap between parts: {} -> {}",
                w[0].end_timestamp_ms,
                w[1].start_timestamp_ms
            );
        }
    }

    /// Segments already under the limit pass through unchanged.
    #[test]
    fn split_short_segment_returns_clone() {
        let segment = SpeechSegment {
            samples: vec![0.1f32; 10 * 16000],
            start_timestamp_ms: 500.0,
            end_timestamp_ms: 10500.0,
            confidence: 0.9,
        };
        let parts = split_segment_at_silence(&segment, 15 * 16000);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].samples.len(), segment.samples.len());
        assert_eq!(parts[0].start_timestamp_ms, 500.0);
        assert_eq!(parts[0].end_timestamp_ms, 10500.0);
    }

    // ── 离线分句兜底切分（2026-09-24）─────────────────────────────────────────
    // 真实数据：Deepgram 批量把 39 秒连续说话（内含十几句）切成**一条 664 字**的 utterance，
    // 落库后成为 transcripts_offline.json 里的巨型段落（详见 split_long_entry 注释）。

    #[test]
    fn long_offline_entry_is_split_at_sentence_boundaries() {
        // 用真实那条 664 字段落的开头部分构造（含 "Eight." 这类上游错听，不影响切分）
        let src = "Eight. I know. I'm really sorry about it. And where is Paulo? Send in Paulo. \
Regina, mi. Bonjour. We're so pleased you could make yourself available to be here. I'm sorry. \
I just have to pause there and say how radiant is Julie Andrews. I was 17 years old when we made this, \
and so I hadn't met as many people yet in in my life. And so I knew Gary Marshall was really special, \
and I knew Julie Andrews was really special. But now sitting here watching this from this point of view, \
they are two of the most magical people I have ever met. This was the film that changed my life.";
        let entry = TranscriptEntry {
            text: src.to_string(),
            start_ms: 10_980.0,
            end_ms: 49_755.0,
            speaker: "1".to_string(),
        };
        assert!(src.chars().count() > OFFLINE_MAX_ENTRY_CHARS, "用例本身要够长");

        let parts = split_long_entry(&entry);
        assert!(parts.len() >= 3, "664 字应被切成多段，实际 {}", parts.len());
        for p in &parts {
            assert!(
                p.text.chars().count() <= OFFLINE_MAX_ENTRY_CHARS + 40,
                "切分后仍有过长段落：{} 字",
                p.text.chars().count()
            );
            assert_eq!(p.speaker, "1", "说话人应沿用原值");
        }
        // 时间戳单调不减、且落在原区间内
        let mut prev_end = entry.start_ms;
        for p in &parts {
            assert!(p.start_ms >= entry.start_ms - 1e-6, "起点越界");
            assert!(p.start_ms >= prev_end - 1e-6, "时间戳应单调不减");
            assert!(p.end_ms >= p.start_ms, "结束应不早于开始");
            assert!(p.end_ms <= entry.end_ms + 1e-6, "终点越界");
            prev_end = p.end_ms;
        }
        // 内容不丢：把标点/空白归一化后应完全一致
        let norm = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>();
        assert_eq!(norm(&parts.iter().map(|p| p.text.clone()).collect::<Vec<_>>().join(" ")), norm(src));
    }

    #[test]
    fn short_offline_entry_is_untouched() {
        let entry = TranscriptEntry {
            text: "Hello. I'm Anne Hathaway.".to_string(),
            start_ms: 0.0,
            end_ms: 2_000.0,
            speaker: String::new(),
        };
        let parts = split_long_entry(&entry);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].text, entry.text);
        assert_eq!(parts[0].start_ms, 0.0);
        assert_eq!(parts[0].end_ms, 2_000.0);
    }

    #[test]
    fn long_entry_without_sentence_punctuation_is_kept() {
        // 没有任何句末标点 → 宁可不切，也不要在词中间断开
        let entry = TranscriptEntry {
            text: "word ".repeat(120).trim().to_string(),
            start_ms: 0.0,
            end_ms: 60_000.0,
            speaker: String::new(),
        };
        let parts = split_long_entry(&entry);
        assert_eq!(parts.len(), 1, "无标点不应强行切分");
    }
}
