mod drift;
#[cfg(test)]
mod tests;

pub use drift::Drift;

use super::fill::Fill;
use super::raster::{self, RoundedRect};
use super::{Canvas, PixelRect, Pose};
use crate::config::{Edge, VisualizerConfig};

const MIN_DIAMETER: f32 = 2.0;
const MIN_RADIUS: f32 = 1.0;
const MIN_VALUE: f32 = 0.01;

/// dot geometry for one buffer size, in buffer pixels
#[derive(Debug, Clone, PartialEq)]
pub struct DotLayout {
    size: (u32, u32),
    along_x: bool,
    spans: Vec<(u32, u32)>,
    base: f32,
    travel: f32,
    min_radius: f32,
}

impl DotLayout {
    pub fn new(
        visualizer: &VisualizerConfig,
        edge: Edge,
        size: (u32, u32),
        scale: f32,
        bars: usize,
        drifting: bool,
    ) -> Self {
        let along_x = matches!(edge, Edge::Bottom | Edge::Top);
        let (length, depth) = if along_x {
            (size.0 as f32, size.1 as f32)
        } else {
            (size.1 as f32, size.0 as f32)
        };
        // legacy takes `bar_width` as the largest radius
        let largest = visualizer.bar_width.max(1) as f32 * 2.0 * scale;
        let gap = visualizer.gap as f32 * scale;
        let room = if drifting { depth * 0.5 } else { depth };
        let count = bars as f32;
        let nominal = count * largest + (count - 1.0).max(0.0) * gap;
        let mut fit = if nominal > length {
            length / nominal
        } else {
            1.0
        };
        if largest * fit > room {
            fit = fit.min(room / largest);
        }
        let diameter = (largest * fit).max(MIN_DIAMETER * scale);
        let gap = gap * fit;
        let spans = spans(bars, length, diameter, gap);

        let largest_radius = diameter * 0.5;
        let reach = (depth - diameter).max(0.0);
        let (base, travel) = match (drifting, edge) {
            (false, _) => (depth * 0.5, 0.0),
            (true, Edge::Top | Edge::Left) => (largest_radius, reach),
            (true, Edge::Bottom | Edge::Right) => (depth - largest_radius, -reach),
        };
        Self {
            size,
            along_x,
            spans,
            base,
            travel,
            min_radius: MIN_RADIUS * scale,
        }
    }

    pub fn pose(&self, index: usize, value: f32, lift: f32) -> Pose {
        let half = self
            .spans
            .get(index)
            .map_or(0.0, |(start, end)| (end - start) as f32 * 0.5);
        let radius = (half * value.clamp(MIN_VALUE, 1.0)).max(self.min_radius);
        Pose {
            extent: radius.min(half),
            shift: self.base + lift.clamp(0.0, 1.0) * self.travel,
        }
    }

    /// every pixel of dot index at pose
    pub fn area(&self, index: usize, pose: Pose) -> Option<PixelRect> {
        let circle = self.circle(index, pose)?;
        PixelRect::covering(
            (circle.left, circle.top, circle.right, circle.bottom),
            self.size,
        )
    }

    /// every pixel of dot index at either pose
    pub fn cover(&self, index: usize, a: Pose, b: Pose) -> Option<PixelRect> {
        match (self.area(index, a), self.area(index, b)) {
            (Some(a), Some(b)) => Some(a.union(b)),
            (a, b) => a.or(b),
        }
    }

    pub fn paint(
        &self,
        canvas: &mut Canvas<'_>,
        index: usize,
        color: usize,
        pose: Pose,
        clip: PixelRect,
        fill: &Fill,
    ) {
        if let Some(circle) = self.circle(index, pose) {
            raster::fill(canvas, circle, clip, fill, color);
        }
    }

    /// dot index at pose as a square rounded into a circle
    fn circle(&self, index: usize, pose: Pose) -> Option<RoundedRect> {
        let &(start, end) = self.spans.get(index)?;
        let along = (start + end) as f32 * 0.5;
        let radius = pose.extent;
        let (x, y) = if self.along_x {
            (along, pose.shift)
        } else {
            (pose.shift, along)
        };
        (radius > 0.0).then_some(RoundedRect {
            left: x - radius,
            top: y - radius,
            right: x + radius,
            bottom: y + radius,
            radius,
        })
    }
}

fn spans(bars: usize, length: f32, diameter: f32, gap: f32) -> Vec<(u32, u32)> {
    let count = bars as f32;
    let rendered = count * diameter + (count - 1.0).max(0.0) * gap;
    let start = (length - rendered).max(0.0) * 0.5;
    let mut spans = Vec::with_capacity(bars);
    let mut previous_end = 0;
    for index in 0..bars {
        let from = start + index as f32 * (diameter + gap);
        let first = (from.round().max(0.0) as u32).max(previous_end);
        let end = ((from + diameter).round() as u32)
            .max(first + 1)
            .min(length as u32);
        previous_end = end;
        spans.push((first.min(end), end));
    }
    spans
}
