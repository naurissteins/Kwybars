use std::f32::consts::{FRAC_PI_2, PI};

use super::placement;
use crate::config::VisualizerConfig;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn visualizer(edit: impl FnOnce(&mut VisualizerConfig)) -> VisualizerConfig {
    let mut visualizer = VisualizerConfig {
        bar_width: 8,
        gap: 0,
        polygon_sides: 3,
        polygon_radius: 180,
        polygon_bar_length: 0,
        polygon_rotation: -90.0,
        polygon_rotation_speed: 0.0,
        ..VisualizerConfig::default()
    };
    edit(&mut visualizer);
    visualizer
}

// legacy `polygon_layout_distributes_bars_across_multiple_edges`
#[test]
fn bars_spread_over_every_side() {
    let placed = placement(&visualizer(|_| {}), (800.0, 800.0), 3);
    let mut angles: Vec<i32> = placed
        .spokes
        .iter()
        .map(|spoke| (spoke.angle.to_degrees() * 10.0).round() as i32)
        .collect();
    angles.sort_unstable();
    angles.dedup();
    assert_eq!(angles.len(), 3);
}

// legacy `polygon_bar_length_caps_outward_motion`
#[test]
fn bar_length_caps_how_far_bars_reach() {
    let capped = placement(
        &visualizer(|v| v.polygon_bar_length = 80),
        (800.0, 800.0),
        1,
    );
    assert_eq!(capped.max_length, 80.0);
    // without it: outer radius 400 - 8 = 392, less the apothem of 90
    let free = placement(&visualizer(|_| {}), (800.0, 800.0), 1);
    assert!(close(free.max_length, 302.0));
    let long = placement(
        &visualizer(|v| v.polygon_bar_length = 5000),
        (800.0, 800.0),
        1,
    );
    assert!(close(long.max_length, 302.0));
}

#[test]
fn bars_start_on_their_side_and_face_away_from_the_center() {
    // a square with a corner up: sides face the diagonals
    let v = visualizer(|v| {
        v.polygon_sides = 4;
        v.polygon_radius = 100;
    });
    let placed = placement(&v, (800.0, 800.0), 4);
    // the first side runs from the top corner to the right one, facing up
    // and to the right
    assert!(close(placed.spokes[0].angle, -PI / 4.0));
    assert!(close(placed.spokes[1].angle, PI / 4.0));
    for spoke in &placed.spokes {
        // on a side: |x| + |y| is the radius all along this square
        assert!(close(spoke.base.0.abs() + spoke.base.1.abs(), 100.0));
        // pointing outwards
        let (x, y) = (spoke.angle.cos(), spoke.angle.sin());
        assert!(spoke.base.0 * x + spoke.base.1 * y > 0.0);
    }
    // four bars one side apart: one per side, all at the same place along it
    let along: Vec<i32> = placed
        .spokes
        .iter()
        .map(|spoke| spoke.base.0.abs().min(spoke.base.1.abs()).round() as i32)
        .collect();
    assert_eq!(along, vec![along[0]; 4]);
}

#[test]
fn crowded_bars_shrink_to_fit_the_perimeter() {
    let v = visualizer(|v| {
        v.polygon_sides = 4;
        v.polygon_radius = 100;
        v.gap = 12;
    });
    // the sides are 141.4 long: 565.7 around, and 100 bars need 2000
    let placed = placement(&v, (800.0, 800.0), 100);
    assert!(close(placed.thickness, 8.0 * 565.685 / 2000.0));
    let roomy = placement(&v, (800.0, 800.0), 4);
    assert_eq!(roomy.thickness, 8.0);
}

#[test]
fn the_radius_leaves_room_and_rotation_turns_the_corners() {
    let wide = placement(&visualizer(|v| v.polygon_radius = 9000), (800.0, 600.0), 1);
    // outer radius 300 - 8 = 292, the corners 10 inside it
    let corner = wide.spokes[0].base;
    assert!(corner.0.hypot(corner.1) <= 282.0 + 1e-3);
    let upright = placement(&visualizer(|v| v.polygon_sides = 4), (800.0, 800.0), 4);
    let flat = placement(
        &visualizer(|v| {
            v.polygon_sides = 4;
            v.polygon_rotation = -45.0;
        }),
        (800.0, 800.0),
        4,
    );
    assert!(close(
        flat.spokes[0].angle - upright.spokes[0].angle,
        FRAC_PI_2 / 2.0
    ));
    assert_eq!(flat.speed, 0.0);
}
