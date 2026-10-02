//! which frequency band each bar shows

use crate::config::BarOrder;

/// bands needed to fill `bars` bars
pub fn band_count(order: BarOrder, bars: usize) -> usize {
    match order {
        BarOrder::LowToHigh => bars,
        BarOrder::BassCenter | BarOrder::BassEdges => bars.div_ceil(2),
    }
}

pub fn band_of(order: BarOrder, bar: usize, bars: usize) -> usize {
    // bars from the middle of the row, 0 for the one or two middle bars
    let from_middle = if 2 * bar + 1 < bars {
        (bars - 1 - 2 * bar) / 2
    } else {
        (2 * bar + 1 - bars) / 2
    };
    match order {
        BarOrder::LowToHigh => bar,
        BarOrder::BassCenter => from_middle,
        BarOrder::BassEdges => (band_count(order, bars) - 1).saturating_sub(from_middle),
    }
}

#[cfg(test)]
mod tests {
    use super::{band_count, band_of};
    use crate::config::BarOrder;

    fn row(order: BarOrder, bars: usize) -> Vec<usize> {
        (0..bars).map(|bar| band_of(order, bar, bars)).collect()
    }

    #[test]
    fn low_to_high_is_one_band_per_bar() {
        assert_eq!(band_count(BarOrder::LowToHigh, 5), 5);
        assert_eq!(row(BarOrder::LowToHigh, 5), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn bass_center_mirrors_around_the_middle() {
        assert_eq!(row(BarOrder::BassCenter, 6), [2, 1, 0, 0, 1, 2]);
        assert_eq!(row(BarOrder::BassCenter, 5), [2, 1, 0, 1, 2]);
        assert_eq!(row(BarOrder::BassCenter, 1), [0]);
    }

    #[test]
    fn bass_edges_is_the_other_way_round() {
        assert_eq!(row(BarOrder::BassEdges, 6), [0, 1, 2, 2, 1, 0]);
        assert_eq!(row(BarOrder::BassEdges, 5), [0, 1, 2, 1, 0]);
        assert_eq!(row(BarOrder::BassEdges, 1), [0]);
    }

    #[test]
    fn every_band_is_shown_and_none_is_out_of_range() {
        for order in [
            BarOrder::LowToHigh,
            BarOrder::BassCenter,
            BarOrder::BassEdges,
        ] {
            for bars in 1..40 {
                let bands = band_count(order, bars);
                let shown = row(order, bars);
                assert!(
                    (0..bands).all(|band| shown.contains(&band)),
                    "{order:?} {bars}"
                );
                assert!(shown.iter().all(|band| *band < bands), "{order:?} {bars}");
            }
        }
    }
}
