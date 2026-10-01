//! bars pointing out from places around a center, optionally turning: the
//! drawing shared by radial and polygon

pub mod polygon;
pub mod radial;
#[cfg(test)]
mod tests;

use super::fill::Fill;
use super::raster::{self, Turned};
use super::{Canvas, PixelRect, Pose};
use crate::config::VisualizerConfig;

/// shortest bar in logical pixels, as in the legacy overlay
const MIN_EXTENT: f32 = 2.0;

/// where a bar starts, from the center, and the angle it points along,
/// before any turn
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spoke {
    pub base: (f32, f32),
    pub angle: f32,
}

/// where a layout puts its bars, in logical pixels
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub spokes: Vec<Spoke>,
    pub thickness: f32,
    pub max_length: f32,
    /// degrees per second
    pub speed: f32,
}

/// in buffer pixels
#[derive(Debug, Clone, PartialEq)]
pub struct Spokes {
    size: (u32, u32),
    center: (f32, f32),
    spokes: Vec<Spoke>,
    max_length: f32,
    min_extent: f32,
    thickness: f32,
    radius: f32,
    segments: Option<(f32, f32)>,
    /// radians per second
    speed: f32,
}

/// the radius bars may reach in a surface of logical size, as legacy
/// centered layouts keep clear of its edges
pub fn outer_radius(visualizer: &VisualizerConfig, (width, height): (f32, f32)) -> f32 {
    let padding = (visualizer.bar_width.max(1) as f32).max(2.0) + visualizer.gap as f32;
    ((width * 0.5).min(height * 0.5) - padding).max(10.0)
}

impl Spokes {
    pub fn new(
        visualizer: &VisualizerConfig,
        size: (u32, u32),
        scale: f32,
        placement: Placement,
    ) -> Self {
        let segment_length = visualizer.segment_length.max(1) as f32 * scale;
        let scaled = |spoke: Spoke| Spoke {
            base: (spoke.base.0 * scale, spoke.base.1 * scale),
            angle: spoke.angle,
        };
        Self {
            size,
            center: (
                size.0 as f32 * 0.5 + visualizer.center_offset_x * scale,
                size.1 as f32 * 0.5 + visualizer.center_offset_y * scale,
            ),
            spokes: placement.spokes.into_iter().map(scaled).collect(),
            max_length: placement.max_length * scale,
            min_extent: MIN_EXTENT * scale,
            thickness: placement.thickness * scale,
            radius: visualizer.bar_corner_radius.max(0.0) * scale,
            segments: visualizer
                .segmented_bars
                .then_some((segment_length, visualizer.segment_gap as f32 * scale)),
            speed: placement.speed.to_radians(),
        }
    }

    pub fn turning(&self) -> bool {
        self.speed != 0.0
    }

    /// radians turned seconds after the bars first showed
    pub fn turn(&self, seconds: f64) -> f32 {
        (seconds * f64::from(self.speed)).rem_euclid(std::f64::consts::TAU) as f32
    }

    pub fn pose(&self, value: f32, turn: f32) -> Pose {
        Pose {
            extent: (value.clamp(0.0, 1.0) * self.max_length).max(self.min_extent),
            shift: turn,
        }
    }

    pub fn area(&self, index: usize, pose: Pose) -> Option<PixelRect> {
        self.shape(index, 0.0, pose.extent, pose.shift)?
            .area(self.size)
    }

    pub fn change(&self, index: usize, old: Pose, new: Pose) -> Option<PixelRect> {
        if old.shift != new.shift {
            return self.cover(index, old, new);
        }
        if old.extent == new.extent {
            return None;
        }
        let from = self.stable(old.extent).min(self.stable(new.extent));
        self.shape(index, from, old.extent.max(new.extent), new.shift)?
            .area(self.size)
    }

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
        let mut piece = |from: f32, to: f32| {
            if let Some(shape) = self.shape(index, from, to, pose.shift) {
                raster::fill_turned(canvas, shape, clip, fill, color);
            }
        };
        match self.segments {
            None => piece(0.0, pose.extent),
            Some((length, gap)) => {
                let mut start = 0.0;
                while start < pose.extent {
                    piece(start, (start + length).min(pose.extent));
                    start += length + gap;
                }
            }
        }
    }

    /// about how many pixels a bar touches at pose
    pub fn footprint(&self, pose: Pose) -> u64 {
        ((pose.extent + 2.0) * (self.thickness + 2.0)) as u64
    }

    /// makes every pixel bar index can touch at pose transparent
    pub fn clear(&self, canvas: &mut Canvas<'_>, index: usize, pose: Pose) {
        if let Some(shape) = self.shape(index, 0.0, pose.extent, pose.shift) {
            raster::clear_turned(canvas, shape);
        }
    }

    /// distance from a bar's start below which it looks the same at extent
    /// and at any longer extent
    fn stable(&self, extent: f32) -> f32 {
        if let Some((length, gap)) = self.segments {
            let step = length + gap;
            return (extent / step).floor() * step;
        }
        let radius = self.radius.min(self.thickness * 0.5);
        if radius <= 0.0 {
            extent
        } else if extent >= 2.0 * radius {
            extent - radius
        } else {
            0.0
        }
    }

    /// bar index between from and to out from its start, the whole layout
    /// turned about the center
    fn shape(&self, index: usize, from: f32, to: f32, turn: f32) -> Option<Turned> {
        let spoke = self.spokes.get(index)?;
        let (sin, cos) = turn.sin_cos();
        let base = (
            spoke.base.0 * cos - spoke.base.1 * sin,
            spoke.base.0 * sin + spoke.base.1 * cos,
        );
        let angle = spoke.angle + turn;
        let axis = (angle.cos(), angle.sin());
        Some(Turned {
            start: (
                self.center.0 + base.0 + axis.0 * from,
                self.center.1 + base.1 + axis.1 * from,
            ),
            axis,
            length: to - from,
            half_width: self.thickness * 0.5,
            radius: self.radius,
        })
    }
}
