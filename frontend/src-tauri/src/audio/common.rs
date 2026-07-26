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

/// Create transcript segments from transcription results.
/// Each tuple is (text, start_ms, end_ms) from VAD timestamps.
pub(crate) fn create_transcript_segments(transcripts: &[(String, f64, f64)]) -> Vec<TranscriptSegment> {
    transcripts
        .iter()
        .map(|(text, start_ms, end_ms)| {
            let start_seconds = start_ms / 1000.0;
            let end_seconds = end_ms / 1000.0;
            let duration = end_seconds - start_seconds;

            TranscriptSegment {
                id: format!("transcript-{}", Uuid::new_v4()),
                text: text.trim().to_string(),
                timestamp: Some(chrono::Utc::now().to_rfc3339()),
                display_time: None,
                audio_start_time: Some(start_seconds),
                audio_end_time: Some(end_seconds),
                duration: Some(duration),
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
                "sequence_id": i
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

    let ms_per_sample = (segment.end_timestamp_ms - segment.start_timestamp_ms)
        / segment.samples.len() as f64;
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
    use super::split_segment_at_silence;
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
}
