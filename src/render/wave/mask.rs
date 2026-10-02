use std::f32::consts::FRAC_1_SQRT_2;

use crate::render::PixelRect;
use crate::render::raster::edge::circle;
use crate::render::raster::{Scan, Turned};

const JOIN_GAP: f32 = 0.05;

/// how much of each pixel a stroke covers, out of 255
#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    data: Vec<u8>,
    size: (u32, u32),
    drawn: Option<PixelRect>,
}

impl Mask {
    pub fn new(size: (u32, u32)) -> Self {
        Self {
            data: vec![0; size.0 as usize * size.1 as usize],
            size,
            drawn: None,
        }
    }

    #[cfg(test)]
    pub fn drawn(&self) -> Option<PixelRect> {
        self.drawn
    }

    /// share of pixel x, y the stroke covers, out of 255
    #[inline]
    pub fn at(&self, x: u32, y: u32) -> u32 {
        if x >= self.size.0 {
            return 0;
        }
        let at = y as usize * self.size.0 as usize + x as usize;
        self.data.get(at).copied().map_or(0, u32::from)
    }

    /// replaces what the mask held with a stroke of width along line, with
    /// round ends and joins
    pub fn stroke(&mut self, line: &[(f32, f32)], width: f32) {
        self.clear();
        let half = width * 0.5;
        let (Some(first), Some(last)) = (line.first(), line.last()) else {
            return;
        };
        let mut before: Option<(f32, f32)> = None;
        for (from, to) in line.iter().zip(line.iter().skip(1)) {
            let (dx, dy) = (to.0 - from.0, to.1 - from.1);
            let length = dx.hypot(dy);
            if length < 1e-3 {
                continue;
            }
            let axis = (dx / length, dy / length);
            if let Some(before) = before
                && half * (before.0 * axis.1 - before.1 * axis.0).abs() > JOIN_GAP
            {
                self.disc(*from, half);
            }
            before = Some(axis);
            self.piece(Turned {
                start: *from,
                axis,
                length,
                half_width: half,
                radius: 0.0,
            });
        }
        self.disc(*first, half);
        self.disc(*last, half);
    }

    fn clear(&mut self) {
        if let Some(area) = self.drawn.take() {
            for y in area.y..area.bottom() {
                self.span(y, area.x, area.right()).fill(0);
            }
        }
    }

    fn span(&mut self, y: u32, x0: u32, x1: u32) -> &mut [u8] {
        let row = y as usize * self.size.0 as usize;
        let (x0, x1) = (x0.min(self.size.0), x1.min(self.size.0));
        self.data
            .get_mut(row + x0 as usize..row + x1.max(x0) as usize)
            .unwrap_or(&mut [])
    }

    fn touch(&mut self, area: PixelRect) {
        self.drawn = Some(self.drawn.map_or(area, |drawn| drawn.union(area)));
    }

    /// covers a straight piece, keeping what is covered more already
    fn piece(&mut self, shape: Turned) {
        let full = PixelRect::full(self.size);
        let (Some(scan), Some(area)) = (Scan::open(&shape, full, self.size), shape.area(self.size))
        else {
            return;
        };
        self.touch(area);
        for y in scan.rows() {
            let row = scan.row(y);
            let span = self.span(y, row.start, row.end);
            let (before, rest) = span.split_at_mut((row.inner_start - row.start) as usize);
            let (inner, after) = rest.split_at_mut((row.inner_end - row.inner_start) as usize);
            let edges = before
                .iter_mut()
                .zip(row.start..)
                .chain(after.iter_mut().zip(row.inner_end..));
            for (pixel, x) in edges {
                let share = (scan.cover(x, y).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                *pixel = (*pixel).max(share);
            }
            inner.fill(u8::MAX);
        }
    }

    /// covers a disc, keeping what is covered more already
    fn disc(&mut self, center: (f32, f32), radius: f32) {
        let reach = radius + FRAC_1_SQRT_2;
        let bounds = (
            center.0 - reach,
            center.1 - reach,
            center.0 + reach,
            center.1 + reach,
        );
        let Some(area) = PixelRect::covering(bounds, self.size) else {
            return;
        };
        self.touch(area);
        let whole = (radius - FRAC_1_SQRT_2).max(0.0);
        for y in area.y..area.bottom() {
            let dy = y as f32 + 0.5 - center.1;
            let chord = |r: f32| (r * r - dy * dy).max(0.0).sqrt();
            // columns whose centers are within the chord
            let columns = |half: f32| {
                // clamped into the area first, so a cast rounds down
                let (low, high) = (area.x as f32, area.right() as f32);
                let first = (center.0 - half - 0.5).clamp(low, high);
                let end = (center.0 + half + 0.5).clamp(low, high) as u32;
                let first = first as u32 + u32::from((first as u32 as f32) < first);
                (first.min(end), end)
            };
            let (start, end) = columns(chord(reach));
            let (inner_start, inner_end) = if dy.abs() < whole {
                let (a, b) = columns(chord(whole));
                (a.clamp(start, end), b.clamp(start, end))
            } else {
                (end, end)
            };
            let span = self.span(y, start, end);
            let (before, rest) = span.split_at_mut((inner_start - start) as usize);
            let (inner, after) = rest.split_at_mut((inner_end - inner_start) as usize);
            let edges = before
                .iter_mut()
                .zip(start..)
                .chain(after.iter_mut().zip(inner_end..));
            for (pixel, x) in edges {
                // most of a join lies under the pieces it joins
                if *pixel == u8::MAX {
                    continue;
                }
                let share = circle(x as f32 + 0.5 - center.0, dy, radius);
                *pixel = (*pixel).max((share.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
            }
            inner.fill(u8::MAX);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Mask;

    fn total(mask: &Mask) -> f32 {
        mask.data
            .iter()
            .map(|share| f32::from(*share) / 255.0)
            .sum()
    }

    #[test]
    fn a_straight_stroke_covers_its_band_and_round_ends() {
        let mut mask = Mask::new((80, 40));
        mask.stroke(&[(20.0, 20.0), (60.0, 20.0)], 8.0);
        // 40 x 8 and two half discs of radius 4
        let area = 40.0 * 8.0 + std::f32::consts::PI * 16.0;
        assert!(
            (total(&mask) - area).abs() < 0.02 * area,
            "{}",
            total(&mask)
        );
        assert_eq!(mask.at(40, 20), 255);
        assert_eq!(mask.at(17, 20), 255);
        assert_eq!(mask.at(10, 20), 0);
        assert_eq!(mask.at(40, 10), 0);
    }

    #[test]
    fn pieces_in_line_leave_no_seam_even_when_thin() {
        for width in [1.0, 1.5, 6.0] {
            let mut whole = Mask::new((90, 30));
            whole.stroke(&[(5.3, 7.2), (80.9, 21.4)], width);
            let mut pieces = Mask::new((90, 30));
            let at = |t: f32| (5.3 + 75.6 * t, 7.2 + 14.2 * t);
            let line: Vec<(f32, f32)> = (0..=9).map(|step| at(step as f32 / 9.0)).collect();
            pieces.stroke(&line, width);
            for (index, (a, b)) in whole.data.iter().zip(&pieces.data).enumerate() {
                assert!(
                    a.abs_diff(*b) <= 2,
                    "width {width} pixel {index}: {a} vs {b}"
                );
            }
        }
    }

    #[test]
    fn a_sharp_bend_is_rounded_on_its_outside() {
        let mut mask = Mask::new((60, 60));
        // up to a corner at 30, 10 and down again
        mask.stroke(&[(10.0, 50.0), (30.0, 10.0), (50.0, 50.0)], 10.0);
        // just above the corner, inside the join's disc
        assert_eq!(mask.at(30, 7), 255);
        assert_eq!(mask.at(30, 3), 0);
    }

    #[test]
    fn a_new_stroke_replaces_the_old_one() {
        let mut mask = Mask::new((60, 40));
        mask.stroke(&[(5.0, 5.0), (50.0, 5.0)], 4.0);
        mask.stroke(&[(5.0, 30.0), (50.0, 30.0)], 4.0);
        assert_eq!(mask.at(20, 5), 0);
        assert_eq!(mask.at(20, 30), 255);
        mask.stroke(&[], 4.0);
        assert!(mask.data.iter().all(|share| *share == 0) && mask.drawn().is_none());
    }
}
