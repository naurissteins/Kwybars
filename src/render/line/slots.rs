//! where bars sit along the edge, ported from the legacy `for_each_linear_slot`

/// continuous or split around a center gap
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Continuous,
    Split { center_gap: f32 },
}

pub fn for_each(
    count: usize,
    length: f32,
    thickness: f32,
    gap: f32,
    mode: Mode,
    mut slot: impl FnMut(usize, f32, f32),
) {
    if count == 0 {
        return;
    }
    let center_gap = match mode {
        Mode::Split { center_gap } if count >= 2 => center_gap.max(0.0),
        _ => {
            let n = count as f32;
            let nominal = n * thickness + (n - 1.0).max(0.0) * gap;
            let scale = if nominal > length {
                length / nominal
            } else {
                1.0
            };
            let size = (thickness * scale).max(1.0);
            let gap = gap * scale;
            let rendered = n * size + (n - 1.0).max(0.0) * gap;
            let start = (length - rendered).max(0.0) * 0.5;
            for index in 0..count {
                slot(index, start + index as f32 * (size + gap), size);
            }
            return;
        }
    };

    let left_count = count / 2;
    let right_count = count - left_count;
    let left_gaps = left_count.saturating_sub(1) as f32;
    let right_gaps = right_count.saturating_sub(1) as f32;
    let nominal = count as f32 * thickness + (left_gaps + right_gaps) * gap + center_gap;
    let scale = if nominal > length {
        length / nominal
    } else {
        1.0
    };
    let size = (thickness * scale).max(1.0);
    let gap = gap * scale;
    let center_gap = center_gap * scale;
    let left_rendered = left_count as f32 * size + left_gaps * gap;
    let right_rendered = right_count as f32 * size + right_gaps * gap;
    let start = (length - (left_rendered + center_gap + right_rendered)).max(0.0) * 0.5;
    for index in 0..left_count {
        slot(index, start + index as f32 * (size + gap), size);
    }
    let right_start = start + left_rendered + center_gap;
    for offset in 0..right_count {
        slot(
            left_count + offset,
            right_start + offset as f32 * (size + gap),
            size,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{Mode, for_each};

    fn slots(count: usize, length: f32, mode: Mode) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for_each(count, length, 20.0, 10.0, mode, |_, start, size| {
            out.push((start, size))
        });
        out
    }

    #[test]
    fn continuous_bars_are_centered() {
        // 3 bars of 20 with 2 gaps of 10 = 80, centered in 100
        assert_eq!(
            slots(3, 100.0, Mode::Continuous),
            vec![(10.0, 20.0), (40.0, 20.0), (70.0, 20.0)]
        );
    }

    #[test]
    fn continuous_bars_shrink_to_fit() {
        let out = slots(4, 55.0, Mode::Continuous);
        // nominal 110 in 55: everything at half size
        assert_eq!(
            out,
            vec![(0.0, 10.0), (15.0, 10.0), (30.0, 10.0), (45.0, 10.0)]
        );
    }

    #[test]
    fn split_mode_leaves_center_gap() {
        let out = slots(4, 400.0, Mode::Split { center_gap: 80.0 });
        assert_eq!(out.len(), 4);
        let left_end = out[1].0 + out[1].1;
        assert!(out[2].0 - left_end >= 80.0 - 1e-4);
        // and stays centered: 20+10+20 | 80 | 20+10+20 = 180 in 400
        assert_eq!(out[0].0, 110.0);
    }

    #[test]
    fn split_with_one_bar_is_continuous() {
        assert_eq!(
            slots(1, 100.0, Mode::Split { center_gap: 80.0 }),
            vec![(40.0, 20.0)]
        );
    }

    #[test]
    fn bars_never_get_thinner_than_one() {
        assert!(
            slots(300, 100.0, Mode::Continuous)
                .iter()
                .all(|(_, size)| *size >= 1.0)
        );
    }
}
