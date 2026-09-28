//! log-spaced frequency bands mapped onto fft bins

use std::ops::Range;

/// bin ranges per bar, lowest frequency first
pub fn bin_ranges(
    bars: usize,
    low_hz: f32,
    high_hz: f32,
    sample_rate: u32,
    fft_len: usize,
) -> Vec<Range<usize>> {
    let bars = bars.max(1);
    let last_bin = fft_len / 2;
    let hz_per_bin = sample_rate as f32 / fft_len as f32;
    let nyquist = sample_rate as f32 / 2.0;
    let high = high_hz.clamp(hz_per_bin * 2.0, nyquist);
    let low = low_hz.clamp(hz_per_bin, high / 2.0);
    let ratio = high / low;

    let mut ranges = Vec::with_capacity(bars);
    let mut next_start = (low / hz_per_bin).floor() as usize;
    for index in 0..bars {
        let edge = low * ratio.powf((index + 1) as f32 / bars as f32);
        let start = next_start.min(last_bin);
        let end = ((edge / hz_per_bin).round() as usize).clamp(start + 1, last_bin + 1);
        ranges.push(start..end);
        next_start = end;
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::bin_ranges;

    #[test]
    fn bands_are_contiguous_non_empty_and_in_range() {
        for (bars, rate) in [
            (1, 48000),
            (16, 44100),
            (50, 48000),
            (256, 44100),
            (512, 96000),
        ] {
            let ranges = bin_ranges(bars, 50.0, 10_000.0, rate, 4096);
            assert_eq!(ranges.len(), bars);
            for range in &ranges {
                assert!(!range.is_empty(), "{bars} bars @ {rate}: empty {range:?}");
                assert!(range.end <= 4096 / 2 + 1);
            }
            for pair in ranges.windows(2) {
                assert_eq!(pair[0].end, pair[1].start, "{bars} bars @ {rate}");
            }
        }
    }

    #[test]
    fn edges_follow_the_cutoffs() {
        let ranges = bin_ranges(10, 50.0, 10_000.0, 48000, 4096);
        let hz = 48000.0 / 4096.0;
        let first = ranges
            .first()
            .map(|r| r.start as f32 * hz)
            .unwrap_or_default();
        let last = ranges.last().map(|r| r.end as f32 * hz).unwrap_or_default();
        assert!((40.0..=60.0).contains(&first), "{first}");
        assert!((9_900.0..=10_100.0).contains(&last), "{last}");
    }

    #[test]
    fn high_cutoff_is_capped_at_nyquist() {
        let ranges = bin_ranges(8, 50.0, 40_000.0, 22050, 1024);
        let end = ranges.last().map_or(0, |r| r.end);
        assert!((1024 / 2..=1024 / 2 + 1).contains(&end), "{end}");
    }
}
