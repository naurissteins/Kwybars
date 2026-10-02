#[cfg(test)]
pub mod tests;

use std::f32::consts::TAU;

use super::{Placement, Spoke, outer_radius};
use crate::config::VisualizerConfig;

/// legacy limits, in logical pixels
const MIN_INNER: f32 = 10.0;
const MIN_LENGTH_ROOM: f32 = 6.0;

/// radians and logical pixels
#[derive(Debug, Clone, Copy, PartialEq)]
struct Distribution {
    first: f32,
    step: f32,
    thickness: f32,
}

pub fn placement(visualizer: &VisualizerConfig, size: (f32, f32), bars: usize) -> Placement {
    let outer = outer_radius(visualizer, size);
    let inner = (visualizer.radial_inner_radius.max(1) as f32)
        .max(MIN_INNER)
        .min((outer - MIN_INNER).max(MIN_INNER));
    let spread = distribution(
        bars,
        inner,
        visualizer.bar_width.max(1) as f32,
        visualizer.gap as f32,
        visualizer.radial_start_angle.to_radians(),
        visualizer.radial_arc_degrees.to_radians(),
    );
    let spoke = |index: usize| {
        let angle = spread.first + index as f32 * spread.step;
        Spoke {
            base: (inner * angle.cos(), inner * angle.sin()),
            angle,
        }
    };
    Placement {
        spokes: (0..bars).map(spoke).collect(),
        thickness: spread.thickness,
        max_length: (outer - inner).max(MIN_LENGTH_ROOM),
        speed: visualizer.radial_rotation_speed,
    }
}

/// legacy radial_distribution, a full circle also leaves a gap between the
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
