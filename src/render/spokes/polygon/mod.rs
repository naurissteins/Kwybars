#[cfg(test)]
mod tests;

use std::f32::consts::{PI, TAU};

use super::{Placement, Spoke, outer_radius};
use crate::config::VisualizerConfig;

/// legacy limits, in logical pixels
const MIN_RADIUS: f32 = 10.0;
const MIN_LENGTH_ROOM: f32 = 6.0;
const MIN_BAR_LENGTH: f32 = 2.0;

pub fn placement(visualizer: &VisualizerConfig, size: (f32, f32), bars: usize) -> Placement {
    let sides = visualizer.polygon_sides.max(3) as usize;
    let outer = outer_radius(visualizer, size);
    let radius = (visualizer.polygon_radius.max(1) as f32)
        .max(MIN_RADIUS)
        .min((outer - MIN_RADIUS).max(MIN_RADIUS));
    let apothem = radius * (PI / sides as f32).cos();
    let room = (outer - apothem).max(MIN_LENGTH_ROOM);
    let max_length = match visualizer.polygon_bar_length {
        0 => room,
        length => (length as f32).min(room).max(MIN_BAR_LENGTH),
    };
    let rotation = visualizer.polygon_rotation.to_radians();
    let vertex = |index: usize| {
        let angle = rotation + (index % sides) as f32 * TAU / sides as f32;
        (radius * angle.cos(), radius * angle.sin())
    };
    let side = 2.0 * radius * (PI / sides as f32).sin();
    let perimeter = side * sides as f32;

    let (thickness, step) = spacing(visualizer, bars, perimeter);

    let spoke = |index: usize| {
        let along = (thickness * 0.5 + index as f32 * step) % perimeter;
        let edge = ((along / side.max(1.0)) as usize) % sides;
        let (from, to) = (vertex(edge), vertex(edge + 1));
        let share = (along % side.max(1.0)) / side.max(1.0);
        // a side faces away from the center through its middle
        let (mx, my) = ((from.0 + to.0) * 0.5, (from.1 + to.1) * 0.5);
        Spoke {
            base: (
                from.0 + (to.0 - from.0) * share,
                from.1 + (to.1 - from.1) * share,
            ),
            angle: if mx == 0.0 && my == 0.0 {
                0.0
            } else {
                my.atan2(mx)
            },
        }
    };
    Placement {
        spokes: (0..bars).map(spoke).collect(),
        thickness,
        max_length,
        speed: visualizer.polygon_rotation_speed,
    }
}

/// bar thickness and the distance from one bar to the next around a
/// perimeter, with a gap after every bar, the last one too
fn spacing(visualizer: &VisualizerConfig, bars: usize, perimeter: f32) -> (f32, f32) {
    let count = bars as f32;
    let gaps = if bars <= 1 { 0.0 } else { count };
    let thickness = visualizer.bar_width.max(1) as f32;
    let gap = visualizer.gap as f32;
    let nominal = count * thickness + gaps * gap;
    let fit = if nominal > perimeter {
        perimeter / nominal
    } else {
        1.0
    };
    let thickness = (thickness * fit).max(1.0);
    let occupied = count * thickness + gaps * gap * fit;
    let step = if gaps > 0.0 {
        thickness + gap * fit + (perimeter - occupied).max(0.0) / gaps
    } else {
        thickness
    };
    (thickness, step)
}
