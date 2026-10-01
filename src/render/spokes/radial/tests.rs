use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::{distribution, placement};
use crate::config::VisualizerConfig;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

pub fn visualizer(edit: impl FnOnce(&mut VisualizerConfig)) -> VisualizerConfig {
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
    // outer radius 300 - (8 + 12) = 280
    let placed = placement(&visualizer(|_| {}), (800.0, 600.0), 4);
    assert!(close(placed.max_length, 180.0));
    assert_eq!(placed.thickness, 8.0);
    // four bars from 0: right, down, left, up (y grows downwards)
    let bases: Vec<(i32, i32)> = placed
        .spokes
        .iter()
        .map(|spoke| (spoke.base.0.round() as i32, spoke.base.1.round() as i32))
        .collect();
    assert_eq!(bases, vec![(100, 0), (0, 100), (-100, 0), (0, -100)]);
    assert!(close(placed.spokes[1].angle, FRAC_PI_2));
}

#[test]
fn the_inner_circle_leaves_room_for_the_bars() {
    let far = placement(
        &visualizer(|v| v.radial_inner_radius = 900),
        (800.0, 600.0),
        4,
    );
    // clamped to the outer radius less 10
    assert!(close(far.spokes[0].base.0, 270.0));
    assert!(close(far.max_length, 10.0));
    let tiny = placement(
        &visualizer(|v| v.radial_inner_radius = 1),
        (800.0, 600.0),
        4,
    );
    assert!(close(tiny.spokes[0].base.0, 10.0));
}
