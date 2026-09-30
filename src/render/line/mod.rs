//! a strip of bars along one edge of a region, growing away from it: the
//! whole `line` layout, and each half of `mirror`

mod slots;
mod strip;
#[cfg(test)]
mod tests;

use super::fill::Fill;
use super::raster::{self, RoundedRect};
use super::{Canvas, PixelRect};
use crate::config::{Edge, LineMode, VisualizerConfig};
use slots::Mode;
pub use strip::{Region, Strip};

/// shortest a `line` bar gets, in logical pixels, as in the legacy overlay
const LINE_MIN_EXTENT: f32 = 2.0;

/// bar geometry for one buffer size, in buffer pixels
#[derive(Debug, Clone, PartialEq)]
pub struct LineLayout {
    edge: Edge,
    size: (u32, u32),
    region: Region,
    /// whole pixel where the spans start along the edge
    origin: f32,
    /// along the edge from `origin`
    spans: Vec<(u32, u32)>,
    depth: f32,
    min_extent: f32,
    radius: f32,
    segments: Option<(f32, f32)>,
}

impl LineLayout {
    /// `bars` bars along `edge` of the whole buffer of `size`, as `line` draws
    /// them; `scale` converts the config's logical pixels
    pub fn along(
        visualizer: &VisualizerConfig,
        edge: Edge,
        size: (u32, u32),
        scale: f32,
        bars: usize,
    ) -> Self {
        let strip = Strip {
            edge,
            region: Region::whole(size),
            min_extent: LINE_MIN_EXTENT,
            mode: visualizer.line_mode,
        };
        Self::new(visualizer, strip, size, scale, bars)
    }

    /// `bars` bars in `strip` of a buffer of `size`
    pub fn new(
        visualizer: &VisualizerConfig,
        strip: Strip,
        size: (u32, u32),
        scale: f32,
        bars: usize,
    ) -> Self {
        let Strip { edge, region, .. } = strip;
        let along_x = matches!(edge, Edge::Bottom | Edge::Top);
        let (origin, length, depth) = if along_x {
            (region.x, region.width, region.height)
        } else {
            (region.y, region.height, region.width)
        };
        let length = length.max(0.0) as u32;
        let depth = depth.max(0.0);
        let mode = match strip.mode {
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
            region,
            origin: origin.round(),
            spans,
            depth,
            min_extent: (strip.min_extent * scale).min(depth),
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

    /// draws bar `index` at `extent` in palette color `color`, touching only
    /// pixels inside `clip`, which must be clear
    pub fn paint(
        &self,
        canvas: &mut Canvas<'_>,
        index: usize,
        color: usize,
        extent: f32,
        clip: PixelRect,
        fill: &Fill,
    ) {
        match self.segments {
            None => {
                if let Some(shape) = self.shape(index, 0.0, extent) {
                    raster::fill(canvas, shape, clip, fill, color);
                }
            }
            Some((length, gap)) => {
                let step = length + gap;
                let mut start = 0.0;
                while start < extent {
                    let end = (start + length).min(extent);
                    if let Some(shape) = self.shape(index, start, end) {
                        raster::fill(canvas, shape, clip, fill, color);
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
        let (start, end) = (self.origin + start as f32, self.origin + end as f32);
        let Region {
            x,
            y,
            width,
            height,
        } = self.region;
        let (left, top, right, bottom) = match self.edge {
            Edge::Bottom => (start, y + height - to, end, y + height - from),
            Edge::Top => (start, y + from, end, y + to),
            Edge::Left => (x + from, start, x + to, end),
            Edge::Right => (x + width - to, start, x + width - from, end),
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
