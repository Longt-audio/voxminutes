//! Audio chunking utilities for long-form transcription.
//!
//! The main helper, `split_at_silence`, divides a sample buffer into contiguous
//! chunks of at most `max_duration_secs`, trying to place boundaries at the
//! quietest nearby point (a pause or silence).  This avoids cutting in the
//! middle of words/sentences when a pause is available.

/// Split `samples` into contiguous chunks of at most `max_duration_secs`,
/// searching `search_radius_secs` around each target boundary for the quietest
/// `silence_window_secs` region.  If no quiet point is found, the boundary
/// falls back to the fixed `max_duration_secs` mark.
///
/// The returned ranges are `(start_sample, end_sample)` and are guaranteed to
/// cover the entire input without gaps or overlaps.
pub fn split_at_silence(
    samples: &[f32],
    sample_rate: u32,
    max_duration_secs: f64,
    search_radius_secs: f64,
    silence_window_secs: f64,
    min_duration_secs: f64,
) -> Vec<(usize, usize)> {
    let max_samples = (max_duration_secs * sample_rate as f64) as usize;
    let search_radius = (search_radius_secs * sample_rate as f64) as usize;
    let silence_window = (silence_window_secs * sample_rate as f64) as usize;
    let min_samples = (min_duration_secs * sample_rate as f64) as usize;

    if samples.len() <= max_samples {
        return vec![(0, samples.len())];
    }

    let cum = energy_cumsum(samples);
    let mut ranges = Vec::new();
    let mut start = 0usize;

    while start < samples.len() {
        let remaining = samples.len() - start;
        if remaining <= max_samples {
            ranges.push((start, samples.len()));
            break;
        }

        let target = start + max_samples;
        let search_start = target.saturating_sub(search_radius).max(start);
        let search_end = (target + search_radius).min(samples.len());

        let mut best_end = target;
        let mut best_energy = f64::INFINITY;

        if search_end >= search_start + silence_window {
            for i in search_start..=search_end - silence_window {
                let energy = cum[i + silence_window] - cum[i];
                if energy < best_energy {
                    best_energy = energy;
                    best_end = i + silence_window / 2;
                }
            }
        }

        // Clamp so we never create a degenerate chunk and never exceed the maximum.
        let upper = (start + max_samples).min(samples.len());
        best_end = best_end.clamp(start + min_samples.min(upper - start), upper);

        ranges.push((start, best_end));
        start = best_end;
    }

    // If the final tail is too short, merge it into the previous chunk.
    if ranges.len() >= 2 {
        let (last_start, last_end) = ranges[ranges.len() - 1];
        if last_end - last_start < min_samples {
            let (prev_start, _) = ranges[ranges.len() - 2];
            ranges.pop();
            ranges.pop();
            ranges.push((prev_start, samples.len()));
        }
    }

    ranges
}

/// Cumulative sum of squared samples for fast window-energy queries.
fn energy_cumsum(samples: &[f32]) -> Vec<f64> {
    let mut cum = Vec::with_capacity(samples.len() + 1);
    cum.push(0.0);
    let mut acc = 0.0;
    for &s in samples {
        let d = s as f64;
        acc += d * d;
        cum.push(acc);
    }
    cum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_for_short_audio() {
        let samples = vec![0.5f32; 16000]; // 1s
        let ranges = split_at_silence(&samples, 16000, 30.0, 1.0, 0.2, 0.5);
        assert_eq!(ranges, vec![(0, 16000)]);
    }

    #[test]
    fn test_boundary_lands_in_silence() {
        // 35s audio with a 200ms silent gap at 30s.
        let mut samples = vec![0.5f32; 35 * 16000];
        let gap_start = 30 * 16000;
        let gap_end = gap_start + 3200;
        for s in &mut samples[gap_start..gap_end] {
            *s = 0.0;
        }

        let ranges = split_at_silence(&samples, 16000, 30.0, 1.0, 0.2, 0.5);
        assert!(ranges.len() >= 2);
        let boundary = ranges[0].1;
        assert!(
            boundary >= gap_start && boundary <= gap_end,
            "boundary {} should be inside silent gap [{}..{}]",
            boundary,
            gap_start,
            gap_end
        );
    }
}
