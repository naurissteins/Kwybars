//! plain bars that prove the paint pipeline; the line layout replaces them

use tiny_skia::Rect;

use crate::config::Edge;

/// share of each bar's cell left empty between bars
const GAP: f32 = 0.3;
/// bars shorter than this many pixels are not drawn
const MIN_EXTENT: f32 = 0.5;

/// bars growing out of one edge of a buffer
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    edge: Edge,
    width: f32,
    height: f32,
    bars: usize,
}

impl Pattern {
    pub fn new(edge: Edge, (width, height): (u32, u32), bars: usize) -> Self {
        Self {
            edge,
            width: width as f32,
            height: height as f32,
            bars: bars.max(1),
        }
    }

    pub fn bar(&self, index: usize, value: f32) -> Option<Rect> {
        let horizontal = matches!(self.edge, Edge::Bottom | Edge::Top);
        let (length, thickness) = if horizontal {
            (self.width, self.height)
        } else {
            (self.height, self.width)
        };
        let extent = value.clamp(0.0, 1.0) * thickness;
        if extent < MIN_EXTENT {
            return None;
        }
        let cell = length / self.bars as f32;
        let start = (index as f32 * cell + cell * GAP / 2.0).round();
        let end = ((index as f32 + 1.0) * cell - cell * GAP / 2.0)
            .round()
            .max(start + 1.0);
        let span = end - start;
        match self.edge {
            Edge::Bottom => Rect::from_xywh(start, self.height - extent, span, extent),
            Edge::Top => Rect::from_xywh(start, 0.0, span, extent),
            Edge::Left => Rect::from_xywh(0.0, start, extent, span),
            Edge::Right => Rect::from_xywh(self.width - extent, start, extent, span),
        }
    }
}

#[cfg(test)]
mod tests {
    use tiny_skia::Rect;

    use super::Pattern;
    use crate::config::Edge;

    #[test]
    fn bars_grow_from_their_edge() {
        let bottom = Pattern::new(Edge::Bottom, (100, 50), 10);
        assert_eq!(bottom.bar(0, 0.5), Rect::from_xywh(2.0, 25.0, 7.0, 25.0));
        assert_eq!(bottom.bar(9, 1.0), Rect::from_xywh(92.0, 0.0, 7.0, 50.0));
        let top = Pattern::new(Edge::Top, (100, 50), 10);
        assert_eq!(top.bar(0, 0.5), Rect::from_xywh(2.0, 0.0, 7.0, 25.0));
        let left = Pattern::new(Edge::Left, (50, 100), 10);
        assert_eq!(left.bar(1, 0.2), Rect::from_xywh(0.0, 12.0, 10.0, 7.0));
        let right = Pattern::new(Edge::Right, (50, 100), 10);
        assert_eq!(right.bar(1, 0.2), Rect::from_xywh(40.0, 12.0, 10.0, 7.0));
    }

    #[test]
    fn silent_bars_are_not_drawn() {
        let pattern = Pattern::new(Edge::Bottom, (100, 50), 10);
        assert_eq!(pattern.bar(3, 0.0), None);
        assert_eq!(pattern.bar(3, 0.009), None);
        assert!(pattern.bar(3, 0.011).is_some());
    }

    #[test]
    fn neighbours_never_share_a_pixel() {
        for (width, bars) in [(1000, 256), (97, 30), (2520, 50), (300, 290)] {
            let pattern = Pattern::new(Edge::Bottom, (width, 10), bars);
            let mut previous_end = 0.0_f32;
            for index in 0..bars {
                let Some(bar) = pattern.bar(index, 1.0) else {
                    panic!("bar {index} missing");
                };
                assert!(
                    bar.left() >= previous_end,
                    "{width} px, {bars} bars, bar {index}"
                );
                assert!(bar.left().fract() == 0.0 && bar.right().fract() == 0.0);
                previous_end = bar.right();
            }
        }
    }
}
