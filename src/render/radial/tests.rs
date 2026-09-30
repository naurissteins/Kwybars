use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::{RadialLayout, distribution};
use crate::config::VisualizerConfig;
use crate::render::Pose;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn visualizer(edit: impl FnOnce(&mut VisualizerConfig)) -> VisualizerConfig {
    let mut visualizer = VisualizerConfig {
        bar_width: 8,
        gap: 12,
        radial_inner_radius: 100,
        radial_start_angle: 0.0,
        radial_arc_degrees: 360.0,
        radial_rotation_speed: 0.0,
        bar_corner_radius: 0.0,
        ..VisualizerConfig::default()
    };
    edit(&mut visualizer);
    visualizer
}

// legacy `radial_partial_arc_stays_inside_requested_span`
#[test]
fn a_partial_arc_keeps_the_bars_inside_it() {
    let spread = distribution(5, 100.0, 8.0, 12.0, -PI, PI);
    let half_bar = spread.thickness * 0.5 / 100.0;
    assert!(close(spread.first, -PI + half_bar));
    assert!(close(spread.first + 4.0 * spread.step, -half_bar));
}

// legacy `radial_single_bar_centers_inside_partial_arc`
#[test]
fn a_single_bar_sits_in_the_middle_of_its_arc() {
    let spread = distribution(1, 120.0, 8.0, 12.0, -FRAC_PI_2, PI);
    assert!(close(spread.first, 0.0));
    assert_eq!(spread.step, 0.0);
}

// legacy `radial_full_circle_clamps_oversized_arc`
#[test]
fn an_oversized_arc_is_a_full_circle() {
    let spread = distribution(4, 100.0, 8.0, 12.0, -FRAC_PI_2, TAU * 2.0);
    assert!(close(spread.first, -FRAC_PI_2));
    assert!(close(spread.step, TAU / 4.0));
}

#[test]
fn crowded_bars_shrink_to_fit_the_inner_circle() {
    // 100 bars of 8 with gaps of 12 need 2000 on a circle of 628
    let spread = distribution(100, 100.0, 8.0, 12.0, 0.0, TAU);
    assert!(close(spread.thickness, 8.0 * TAU * 100.0 / 2000.0));
    assert!(close(spread.step, TAU / 100.0));
}

#[test]
fn bars_reach_from_the_inner_circle_towards_the_short_side() {
    // 800x600 logical at scale 1.5: outer radius 300 - (8 + 12) = 280
    let layout = RadialLayout::new(&visualizer(|_| {}), (1200, 900), 1.5, 4);
    assert_eq!(layout.center, (600.0, 450.0));
    assert!(close(layout.inner, 150.0));
    assert!(close(layout.max_length, 180.0 * 1.5));
    assert_eq!(layout.pose(1.0, 0.0).extent, layout.max_length);
    // 2 logical pixels at the least
    assert_eq!(layout.pose(0.0, 0.0).extent, 3.0);
}

#[test]
fn the_inner_circle_leaves_room_for_the_bars() {
    let layout = RadialLayout::new(
        &visualizer(|v| v.radial_inner_radius = 900),
        (800, 600),
        1.0,
        4,
    );
    // clamped to the outer radius less 10
    assert!(close(layout.inner, 270.0));
    assert!(close(layout.max_length, 10.0));
    let tiny = RadialLayout::new(
        &visualizer(|v| v.radial_inner_radius = 1),
        (800, 600),
        1.0,
        4,
    );
    assert!(close(tiny.inner, 10.0));
}

#[test]
fn the_center_moves_by_the_offsets() {
    let v = visualizer(|v| {
        v.center_offset_x = 20.0;
        v.center_offset_y = -10.0;
    });
    let layout = RadialLayout::new(&v, (800, 600), 2.0, 4);
    assert_eq!(layout.center, (440.0, 280.0));
}

#[test]
fn bars_point_out_along_their_angle() {
    // four bars from 0: right, down, left, up (y grows downwards)
    let layout = RadialLayout::new(&visualizer(|_| {}), (800, 600), 1.0, 4);
    let pose = layout.pose(0.5, 0.0);
    let Some(right) = layout.area(0, pose) else {
        panic!("bar 0 has no pixels");
    };
    // bounds reach half a pixel past the shape
    assert!(right.x >= 499 && right.y < 300 && right.bottom() > 300);
    let Some(up) = layout.area(3, pose) else {
        panic!("bar 3 has no pixels");
    };
    assert!(up.bottom() <= 201 && up.x < 400 && up.right() > 400);
    // a quarter turn moves bar 0 to where bar 1 was
    let turned = layout.area(0, layout.pose(0.5, FRAC_PI_2));
    assert_eq!(turned, layout.area(1, pose));
}

#[test]
fn turning_follows_the_speed_and_wraps() {
    let layout = RadialLayout::new(
        &visualizer(|v| v.radial_rotation_speed = 90.0),
        (800, 600),
        1.0,
        4,
    );
    assert!(layout.turning());
    assert!(close(layout.turn(1.0), FRAC_PI_2));
    assert!(close(layout.turn(5.0), FRAC_PI_2));
    let still = RadialLayout::new(&visualizer(|_| {}), (800, 600), 1.0, 4);
    assert!(!still.turning() && still.turn(12.0) == 0.0);
}

#[test]
fn a_growing_bar_damages_only_its_tip() {
    let layout = RadialLayout::new(&visualizer(|_| {}), (800, 600), 1.0, 4);
    let (short, long) = (layout.pose(0.2, 0.0), layout.pose(0.6, 0.0));
    let Some(change) = layout.change(0, short, long) else {
        panic!("a growing bar changes");
    };
    let Some(whole) = layout.area(0, long) else {
        panic!("bar 0 has no pixels");
    };
    assert!(change.x > whole.x && change.right() == whole.right());
    assert_eq!(layout.change(0, long, long), None);
    let turned = Pose { shift: 0.1, ..long };
    assert_eq!(
        layout.change(0, long, turned),
        layout.cover(0, long, turned)
    );
}
