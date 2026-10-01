use std::f32::consts::FRAC_PI_2;

use super::radial::tests::visualizer;
use super::{Spokes, radial};
use crate::config::VisualizerConfig;
use crate::render::Pose;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

/// a radial ring in a buffer of size
fn ring(visualizer: &VisualizerConfig, size: (u32, u32), scale: f32) -> Spokes {
    let logical = (size.0 as f32 / scale, size.1 as f32 / scale);
    Spokes::new(
        visualizer,
        size,
        scale,
        radial::placement(visualizer, logical, 4),
    )
}

#[test]
fn logical_sizes_scale_to_the_buffer() {
    // 800x600 logical at scale 1.5
    let spokes = ring(&visualizer(|_| {}), (1200, 900), 1.5);
    assert_eq!(spokes.center, (600.0, 450.0));
    assert!(close(spokes.spokes[0].base.0, 150.0));
    assert!(close(spokes.max_length, 180.0 * 1.5));
    assert_eq!(spokes.pose(1.0, 0.0).extent, spokes.max_length);
    // 2 logical pixels at the least
    assert_eq!(spokes.pose(0.0, 0.0).extent, 3.0);
}

#[test]
fn the_center_moves_by_the_offsets() {
    let v = visualizer(|v| {
        v.center_offset_x = 20.0;
        v.center_offset_y = -10.0;
    });
    assert_eq!(ring(&v, (800, 600), 2.0).center, (440.0, 280.0));
}

#[test]
fn bars_point_out_along_their_angle() {
    // four bars from 0: right, down, left, up (y grows downwards)
    let spokes = ring(&visualizer(|_| {}), (800, 600), 1.0);
    let pose = spokes.pose(0.5, 0.0);
    let Some(right) = spokes.area(0, pose) else {
        panic!("bar 0 has no pixels");
    };
    // bounds reach half a pixel past the shape
    assert!(right.x >= 499 && right.y < 300 && right.bottom() > 300);
    let Some(up) = spokes.area(3, pose) else {
        panic!("bar 3 has no pixels");
    };
    assert!(up.bottom() <= 201 && up.x < 400 && up.right() > 400);
    // a quarter turn moves bar 0 to where bar 1 was
    let turned = spokes.area(0, spokes.pose(0.5, FRAC_PI_2));
    assert_eq!(turned, spokes.area(1, pose));
}

#[test]
fn turning_follows_the_speed_and_wraps() {
    let spokes = ring(
        &visualizer(|v| v.radial_rotation_speed = 90.0),
        (800, 600),
        1.0,
    );
    assert!(spokes.turning());
    assert!(close(spokes.turn(1.0), FRAC_PI_2));
    assert!(close(spokes.turn(5.0), FRAC_PI_2));
    let still = ring(&visualizer(|_| {}), (800, 600), 1.0);
    assert!(!still.turning() && still.turn(12.0) == 0.0);
}

#[test]
fn a_growing_bar_damages_only_its_tip() {
    let spokes = ring(&visualizer(|_| {}), (800, 600), 1.0);
    let (short, long) = (spokes.pose(0.2, 0.0), spokes.pose(0.6, 0.0));
    let Some(change) = spokes.change(0, short, long) else {
        panic!("a growing bar changes");
    };
    let Some(whole) = spokes.area(0, long) else {
        panic!("bar 0 has no pixels");
    };
    assert!(change.x > whole.x && change.right() == whole.right());
    assert_eq!(spokes.change(0, long, long), None);
    let turned = Pose { shift: 0.1, ..long };
    assert_eq!(
        spokes.change(0, long, turned),
        spokes.cover(0, long, turned)
    );
}
