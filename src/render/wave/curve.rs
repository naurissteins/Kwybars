/// most straight pieces between two points
pub const MAX_PIECES: usize = 48;
const TOLERANCE: f32 = 0.05;

pub fn flatten(points: &[(f32, f32)], control: f32, out: &mut Vec<(f32, f32)>) {
    out.clear();
    let Some(first) = points.first() else {
        return;
    };
    out.push(*first);
    let at = |index: usize| points.get(index).copied().unwrap_or(*first);
    for index in 0..points.len().saturating_sub(1) {
        let (p1, p2) = (at(index), at(index + 1));
        let p0 = at(index.saturating_sub(1));
        let p3 = at((index + 2).min(points.len() - 1));
        let c1 = (
            p1.0 + (p2.0 - p0.0) * control,
            p1.1 + (p2.1 - p0.1) * control,
        );
        let c2 = (
            p2.0 - (p3.0 - p1.0) * control,
            p2.1 - (p3.1 - p1.1) * control,
        );
        let pieces = pieces(p1, c1, c2, p2);
        for step in 1..=pieces {
            let t = step as f32 / pieces as f32;
            let s = 1.0 - t;
            let (a, b, c, d) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
            out.push((
                a * p1.0 + b * c1.0 + c * c2.0 + d * p2.0,
                a * p1.1 + b * c1.1 + c * c2.1 + d * p2.1,
            ));
        }
    }
}

/// pieces that keep a cubic within the tolerance
fn pieces(p1: (f32, f32), c1: (f32, f32), c2: (f32, f32), p2: (f32, f32)) -> usize {
    let chord = (p2.0 - p1.0, p2.1 - p1.1);
    let length = chord.0.hypot(chord.1).max(f32::EPSILON);
    // how far a control point sits off the straight line between the ends
    let off = |c: (f32, f32)| ((c.0 - p1.0) * chord.1 - (c.1 - p1.1) * chord.0).abs() / length;
    // the curve bows at most 3/4 of that, and a piece of it 1 / pieces^2
    let bow = 0.75 * off(c1).max(off(c2));
    ((bow / TOLERANCE).sqrt().ceil() as usize).clamp(1, MAX_PIECES)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    spans: Vec<(f32, f32)>,
    cells: (usize, usize),
    ends: (f32, f32),
}

const EMPTY: (f32, f32) = (f32::INFINITY, f32::NEG_INFINITY);

impl Profile {
    pub fn new(cells: usize) -> Self {
        Self {
            spans: vec![EMPTY; cells],
            cells: (0, 0),
            ends: (0.0, 0.0),
        }
    }

    /// takes the curve of a polyline given as along, across pairs
    pub fn trace(&mut self, line: impl Iterator<Item = (f32, f32)> + Clone) {
        let (first, last) = self.cells;
        if let Some(spans) = self.spans.get_mut(first..last) {
            spans.fill(EMPTY);
        }
        let (mut low, mut high) = (f32::INFINITY, f32::NEG_INFINITY);
        for (from, to) in line.clone().zip(line.skip(1)) {
            let (from, to) = if from.0 <= to.0 {
                (from, to)
            } else {
                (to, from)
            };
            (low, high) = (low.min(from.0), high.max(to.0));
            self.cross(from, to);
        }
        let cells = self.spans.len();
        if low > high || cells == 0 {
            self.cells = (0, 0);
            return;
        }
        let cell = |along: f32| (along.max(0.0) as usize).min(cells - 1);
        self.cells = (cell(low), cell(high) + 1);
        let (first, last) = (cell(low) as f32, cell(high) as f32);
        self.ends = (
            (first + 1.0 - low).clamp(0.0, 1.0),
            (high - last).clamp(0.0, 1.0),
        );
    }

    /// widens the span of every cell the piece from..to passes
    fn cross(&mut self, from: (f32, f32), to: (f32, f32)) {
        let run = to.0 - from.0;
        let across = |along: f32| {
            if run <= 0.0 {
                from.1
            } else {
                from.1 + (to.1 - from.1) * ((along - from.0) / run)
            }
        };
        let first = from.0.max(0.0) as usize;
        let last = (to.0.max(0.0) as usize).min(self.spans.len().saturating_sub(1));
        for cell in first..=last {
            let Some(span) = self.spans.get_mut(cell) else {
                return;
            };
            let (a, b) = (
                across(from.0.max(cell as f32)),
                across(to.0.min(cell as f32 + 1.0)),
            );
            *span = (span.0.min(a.min(b)), span.1.max(a.max(b)));
        }
        if run <= 0.0
            && let Some(span) = self.spans.get_mut(first)
        {
            *span = (span.0.min(from.1.min(to.1)), span.1.max(from.1.max(to.1)));
        }
    }

    /// cells the curve passes, first to one past the last
    pub fn cells(&self) -> (usize, usize) {
        self.cells
    }

    /// lowest and highest across position in a cell the curve passes
    pub fn span(&self, cell: usize) -> Option<(f32, f32)> {
        let span = *self.spans.get(cell)?;
        (span.0 <= span.1).then_some(span)
    }

    /// how much of a cell, along the axis, the curve passes over
    pub fn share(&self, cell: usize) -> f32 {
        let (first, last) = self.cells;
        if cell < first || cell >= last {
            0.0
        } else if cell == first && cell + 1 == last {
            (self.ends.0 + self.ends.1 - 1.0).max(0.0)
        } else if cell == first {
            self.ends.0
        } else if cell + 1 == last {
            self.ends.1
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_PIECES, Profile, flatten};

    #[test]
    fn a_straight_run_is_one_piece_and_bends_get_more() {
        let mut out = Vec::new();
        flatten(
            &[(0.0, 5.0), (50.0, 5.0), (100.0, 5.0)],
            1.0 / 6.0,
            &mut out,
        );
        assert_eq!(out, vec![(0.0, 5.0), (50.0, 5.0), (100.0, 5.0)]);
        flatten(
            &[(0.0, 50.0), (50.0, 0.0), (100.0, 50.0)],
            1.0 / 6.0,
            &mut out,
        );
        assert!(out.len() > 8 && out.len() <= 2 * MAX_PIECES + 1);
        // through every point, and never past the peak
        assert!(out.contains(&(50.0, 0.0)) && out.last() == Some(&(100.0, 50.0)));
        assert!(out.iter().all(|point| point.1 >= -1e-3));
        // without smoothing the pieces are the straight lines between points
        flatten(&[(0.0, 50.0), (50.0, 0.0), (100.0, 50.0)], 0.0, &mut out);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn pieces_stay_within_a_tenth_of_a_pixel_of_the_curve() {
        let points = [(0.0, 80.0), (40.0, 5.0), (80.0, 90.0), (120.0, 40.0)];
        let control = 1.0 / 3.0;
        let mut out = Vec::new();
        flatten(&points, control, &mut out);
        // the middle span's cubic, sampled finely
        let (p0, p1, p2, p3) = (points[0], points[1], points[2], points[3]);
        let c1 = (
            p1.0 + (p2.0 - p0.0) * control,
            p1.1 + (p2.1 - p0.1) * control,
        );
        let c2 = (
            p2.0 - (p3.0 - p1.0) * control,
            p2.1 - (p3.1 - p1.1) * control,
        );
        for step in 0..=400 {
            let t = step as f32 / 400.0;
            let s = 1.0 - t;
            let (a, b, c, d) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
            let on = (
                a * p1.0 + b * c1.0 + c * c2.0 + d * p2.0,
                a * p1.1 + b * c1.1 + c * c2.1 + d * p2.1,
            );
            let nearest = out
                .windows(2)
                .map(|piece| distance(on, piece[0], piece[1]))
                .fold(f32::MAX, f32::min);
            assert!(nearest < 0.15, "{on:?} is {nearest} from the pieces");
        }
    }

    fn distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let along = ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy).max(1e-9);
        let along = along.clamp(0.0, 1.0);
        (p.0 - a.0 - dx * along).hypot(p.1 - a.1 - dy * along)
    }

    #[test]
    fn the_profile_holds_where_the_curve_passes_each_cell() {
        let mut profile = Profile::new(10);
        // from 1.5 to 4.5 along, rising 1 across per cell
        profile.trace([(1.5, 10.0), (4.5, 13.0)].into_iter());
        assert_eq!(profile.cells(), (1, 5));
        assert_eq!(profile.span(0), None);
        assert_eq!(profile.span(1), Some((10.0, 10.5)));
        assert_eq!(profile.span(2), Some((10.5, 11.5)));
        assert_eq!(profile.span(4), Some((12.5, 13.0)));
        assert_eq!(
            (profile.share(1), profile.share(2), profile.share(4)),
            (0.5, 1.0, 0.5)
        );
        assert_eq!(profile.share(7), 0.0);
        // a new trace forgets the old cells
        profile.trace([(6.0, 2.0), (8.0, 2.0)].into_iter());
        assert_eq!(profile.span(2), None);
        assert_eq!(profile.span(6), Some((2.0, 2.0)));
    }

    #[test]
    fn a_piece_straight_across_widens_its_cell() {
        let mut profile = Profile::new(4);
        profile.trace([(1.25, 3.0), (1.25, 9.0), (2.5, 9.0)].into_iter());
        assert_eq!(profile.span(1), Some((3.0, 9.0)));
        assert_eq!(profile.span(2), Some((9.0, 9.0)));
    }
}
