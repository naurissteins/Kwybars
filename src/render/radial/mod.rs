#[cfg(test)]
mod tests;

use std::f32::consts::TAU;

use super::fill::Fill;
use super::raster::{self, Turned};
use super::{Canvas, PixelRect, Pose};
use crate::config::VisualizerConfig;

/// legacy limits, in logical pixels
const MIN_INNER: f32 = 10.0;
const MIN_LENGTH_ROOM: f32 = 6.0;
const MIN_EXTENT: f32 = 2.0;

/// radians and logical pixels
#[derive(Debug, Clone, Copy, PartialEq)]
struct Distribution {
    first: f32,
    step: f32,
    thickness: f32,
}

/// in buffer pixels
#[derive(Debug, Clone, PartialEq)]
pub struct RadialLayout {
    size: (u32, u32),
    center: (f32, f32),
    inner: f32,
    max_length: f32,
    min_extent: f32,
    thickness: f32,
    radius: f32,
    segments: Option<(f32, f32)>,
    angles: Vec<f32>,
    /// radians per second
    speed: f32,
}

impl RadialLayout {
    pub fn new(visualizer: &VisualizerConfig, size: (u32, u32), scale: f32, bars: usize) -> Self {
        let (width, height) = (size.0 as f32 / scale, size.1 as f32 / scale);
        let thickness = visualizer.bar_width.max(1) as f32;
        let gap = visualizer.gap as f32;
        let outer = ((width * 0.5).min(height * 0.5) - (thickness.max(2.0) + gap)).max(10.0);
        let inner = (visualizer.radial_inner_radius.max(1) as f32)
            .max(MIN_INNER)
            .min((outer - MIN_INNER).max(MIN_INNER));
        let max_length = (outer - inner).max(MIN_LENGTH_ROOM);
        let spread = distribution(
            bars,
            inner,
            thickness,
            gap,
            visualizer.radial_start_angle.to_radians(),
            visualizer.radial_arc_degrees.to_radians(),
        );
        let segment_length = visualizer.segment_length.max(1) as f32 * scale;
        Self {
            size,
            center: (
                size.0 as f32 * 0.5 + visualizer.center_offset_x * scale,
                size.1 as f32 * 0.5 + visualizer.center_offset_y * scale,
            ),
            inner: inner * scale,
            max_length: max_length * scale,
            min_extent: MIN_EXTENT * scale,
            thickness: spread.thickness * scale,
            radius: visualizer.bar_corner_radius.max(0.0) * scale,
            segments: visualizer
                .segmented_bars
                .then_some((segment_length, visualizer.segment_gap as f32 * scale)),
            angles: (0..bars)
                .map(|index| spread.first + index as f32 * spread.step)
                .collect(),
            speed: visualizer.radial_rotation_speed.to_radians(),
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

    /// about how many pixels bar index touches at pose
    pub fn footprint(&self, pose: Pose) -> u64 {
        ((pose.extent + 2.0) * (self.thickness + 2.0)) as u64
    }

    /// makes every pixel bar index can touch at pose transparent
    pub fn clear(&self, canvas: &mut Canvas<'_>, index: usize, pose: Pose) {
        if let Some(shape) = self.shape(index, 0.0, pose.extent, pose.shift) {
            raster::clear_turned(canvas, shape);
        }
    }

    /// distance from the inner circle below which a bar looks the same at
    /// extent and at any longer extent
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

    /// bar index between from and to out from the inner circle
    fn shape(&self, index: usize, from: f32, to: f32, turn: f32) -> Option<Turned> {
        let angle = self.angles.get(index)? + turn;
        let axis = (angle.cos(), angle.sin());
        let base = self.inner + from;
        Some(Turned {
            start: (self.center.0 + axis.0 * base, self.center.1 + axis.1 * base),
            axis,
            length: to - from,
            half_width: self.thickness * 0.5,
            radius: self.radius,
        })
    }
}

/// legacy radial_distribution; a full circle also leaves a gap between the
/// last bar and the first
fn distribution(
    count: usize,
    inner: f32,
    thickness: f32,
    gap: f32,
    start: f32,
    arc: f32,
) -> Distribution {
    let inner = inner.max(1.0);
    let arc = arc.clamp(-TAU, TAU);
    let direction = if arc < 0.0 { -1.0 } else { 1.0 };
    let magnitude = arc.abs().max(0.001);
    let full_circle = (magnitude - TAU).abs() < 0.001;
    let gaps = match count {
        0 | 1 => 0,
        _ if full_circle => count,
        _ => count - 1,
    } as f32;
    let nominal = count as f32 * thickness.max(1.0) + gaps * gap.max(0.0);
    let available = magnitude * inner;
    let fit = if nominal > available {
        available / nominal
    } else {
        1.0
    };
    let thickness = (thickness * fit).max(1.0);
    let base_gap = gap.max(0.0) * fit;
    let occupied = count as f32 * thickness + gaps * base_gap;
    let extra = if gaps > 0.0 {
        (available - occupied).max(0.0) / gaps
    } else {
        0.0
    };
    let step = if count <= 1 {
        0.0
    } else {
        direction * (thickness + base_gap + extra) / inner
    };
    let first = if full_circle {
        start
    } else if count == 1 {
        start + arc * 0.5
    } else {
        start + direction * thickness * 0.5 / inner
    };
    Distribution {
        first,
        step,
        thickness,
    }
}
