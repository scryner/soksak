//! All times are on the original decoded PCM timeline. No silence compression.
use crate::ffmpeg_decoder::SAMPLE_RATE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioWindow {
    pub start: usize,
    pub end: usize,
}

impl AudioWindow {
    pub fn start_cs(self) -> i64 {
        (self.start * 100 / SAMPLE_RATE) as i64
    }
    pub fn end_cs(self) -> i64 {
        (self.end * 100 / SAMPLE_RATE) as i64
    }
}

/// Group nearby speech while retaining the real silence and absolute sample positions.
/// Overlapping VAD ranges are unioned so the same speech is never decoded twice.
pub fn speech_windows(
    ranges_cs: impl IntoIterator<Item = (f32, f32)>,
    samples: usize,
) -> Vec<AudioWindow> {
    let mut ranges: Vec<_> = ranges_cs
        .into_iter()
        .filter_map(|(start, end)| {
            if !start.is_finite() || !end.is_finite() || end <= start {
                return None;
            }
            let start = ((start.max(0.0) as f64 * SAMPLE_RATE as f64 / 100.0).round() as usize)
                .min(samples);
            let end =
                ((end.max(0.0) as f64 * SAMPLE_RATE as f64 / 100.0).round() as usize).min(samples);
            (end > start).then_some(AudioWindow { start, end })
        })
        .collect();
    ranges.sort_by_key(|r| r.start);
    let mut windows: Vec<AudioWindow> = Vec::new();
    for range in ranges {
        if let Some(last) = windows.last_mut() {
            if range.start <= last.end
                || (range.start - last.end <= SAMPLE_RATE * 3 / 4
                    && range.end - last.start <= SAMPLE_RATE * 30)
            {
                last.end = last.end.max(range.end);
                continue;
            }
        }
        windows.push(range);
    }
    windows
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_silence_is_never_compressed_or_interpolated() {
        let windows = speech_windows([(1000.0, 1100.0), (61000.0, 61100.0)], SAMPLE_RATE * 620);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[1].start_cs() + 20, 61020);
        assert_eq!(windows[0].end_cs(), 1100);
    }
    #[test]
    fn nearby_speech_keeps_its_gap_and_overlaps_are_not_duplicated() {
        let windows = speech_windows(
            [(100.0, 200.0), (230.0, 300.0), (280.0, 400.0)],
            SAMPLE_RATE * 5,
        );
        assert_eq!(
            windows,
            vec![AudioWindow {
                start: SAMPLE_RATE,
                end: SAMPLE_RATE * 4
            }]
        );
    }
    #[test]
    fn invalid_and_out_of_range_vad_output_is_bounded() {
        let windows = speech_windows(
            [(f32::NAN, 10.0), (50.0, 10.0), (-10.0, 300.0)],
            SAMPLE_RATE * 2,
        );
        assert_eq!(
            windows,
            vec![AudioWindow {
                start: 0,
                end: SAMPLE_RATE * 2
            }]
        );
    }
}
