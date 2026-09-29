//! the `line` layout: bars along one edge, growing away from it

mod slots;
#[cfg(test)]
mod tests;

use super::fill::Fill;
use super::raster::{self, RoundedRect};
use super::{Canvas, PixelRect};
use crate::config::{Edge, LineMode, VisualizerConfig};
use slots::Mode;

/// shortest a bar gets, in logical pixels, as in the legacy overlay
const MIN_EXTENT: f32 = 2.0;

/// bar geometry for one buffer size, in buffer pixels
#[derive(Debug, Clone, PartialEq)]
pub struct LineLayout {
    edge: Edge,
    size: (u32, u32),
    spans: Vec<(u32, u32)>,
    depth: f32,
    min_extent: f32,
    radius: f32,
    segments: Option<(f32, f32)>,
}

impl LineLayout {
    /// lays out `bars` bars; `scale` converts the config's logical pixels
    pub fn new(
        visualizer: &VisualizerConfig,
        edge: Edge,
        size: (u32, u32),
        scale: f32,
        bars: usize,
    ) -> Self {
        let along_x = matches!(edge, Edge::Bottom | Edge::Top);
        let (length, depth) = if along_x {
            (size.0, size.1)
        } else {
            (size.1, size.0)
        };
        let mode = match visualizer.line_mode {
            LineMode::Continuous => Mode::Continuous,
            LineMode::Split => Mode::Split {
                center_gap: visualizer.line_split_gap as f32 * scale,
            },
        };
        let mut spans = Vec::with_capacity(bars);
        let mut previous_end = 0;
        slots::for_each(
            bars,
            length as f32,
            visualizer.bar_width.max(1) as f32 * scale,
            visualizer.gap as f32 * scale,
            mode,
            |_, start, thickness| {
                let first = (start.round().max(0.0) as u32).max(previous_end);
                let end = ((start + thickness).round() as u32)
                    .max(first + 1)
                    .min(length);
                let first = first.min(end);
                previous_end = end;
                spans.push((first, end));
            },
        );
        let segment_length = visualizer.segment_length.max(1) as f32 * scale;
        Self {
            edge,
            size,
            spans,
            depth: depth as f32,
            min_extent: (MIN_EXTENT * scale).min(depth as f32),
            radius: visualizer.bar_corner_radius.max(0.0) * scale,
            segments: visualizer
                .segmented_bars
                .then_some((segment_length, visualizer.segment_gap as f32 * scale)),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn bars(&self) -> usize {
        self.spans.len()
    }

    /// how far bar `value` in `0.0..=1.0` reaches from the edge, in pixels
    pub fn extent(&self, value: f32) -> f32 {
        (value.clamp(0.0, 1.0) * self.depth).max(self.min_extent)
    }

    /// every pixel of bar `index` at `extent`
    pub fn area(&self, index: usize, extent: f32) -> Option<PixelRect> {
        self.band(index, 0.0, extent)
    }

    /// the pixels that differ between bar `index` at `old` and at `new`
    pub fn change(&self, index: usize, old: f32, new: f32) -> Option<PixelRect> {
        if old == new {
            return None;
        }
        let from = self.stable(index, old).min(self.stable(index, new));
        self.band(index, from, old.max(new))
    }

    /// draws bar `index` at `extent`, touching only pixels inside `clip`,
    /// which must be clear
    pub fn paint(
        &self,
        canvas: &mut Canvas<'_>,
        index: usize,
        extent: f32,
        clip: PixelRect,
        fill: &Fill,
    ) {
        match self.segments {
            None => {
                if let Some(shape) = self.shape(index, 0.0, extent) {
                    raster::fill(canvas, shape, clip, fill, index);
                }
            }
            Some((length, gap)) => {
                let step = length + gap;
                let mut start = 0.0;
                while start < extent {
                    let end = (start + length).min(extent);
                    if let Some(shape) = self.shape(index, start, end) {
                        raster::fill(canvas, shape, clip, fill, index);
                    }
                    start += step;
                }
            }
        }
    }

    /// distance from the edge below which bar `index` looks the same at
    /// `extent` and at any longer extent
    fn stable(&self, index: usize, extent: f32) -> f32 {
        if let Some((length, gap)) = self.segments {
            // the segment holding the tip may be partial, the ones below are whole
            let step = length + gap;
            return (extent / step).floor() * step;
        }
        let radius = self.radius.min(self.thickness(index) * 0.5);
        if radius <= 0.0 {
            extent
        } else if extent >= 2.0 * radius {
            // only the rounded cap moves with the tip
            extent - radius
        } else {
            // the corners at the edge shrink too
            0.0
        }
    }

    fn thickness(&self, index: usize) -> f32 {
        self.spans
            .get(index)
            .map_or(0.0, |(start, end)| (end - start) as f32)
    }

    /// the pixel bounds of bar `index` between `from` and `to` away from the edge
    fn band(&self, index: usize, from: f32, to: f32) -> Option<PixelRect> {
        let shape = self.shape(index, from, to)?;
        PixelRect::covering(
            (shape.left, shape.top, shape.right, shape.bottom),
            self.size,
        )
    }

    /// bar `index` between `from` and `to` away from the edge as a shape
    fn shape(&self, index: usize, from: f32, to: f32) -> Option<RoundedRect> {
        let &(start, end) = self.spans.get(index)?;
        let (start, end) = (start as f32, end as f32);
        let (width, height) = (self.size.0 as f32, self.size.1 as f32);
        let (left, top, right, bottom) = match self.edge {
            Edge::Bottom => (start, height - to, end, height - from),
            Edge::Top => (start, from, end, to),
            Edge::Left => (from, start, to, end),
            Edge::Right => (width - to, start, width - from, end),
        };
        Some(RoundedRect {
            left,
            top,
            right,
            bottom,
            radius: self.radius,
        })
    }
}
