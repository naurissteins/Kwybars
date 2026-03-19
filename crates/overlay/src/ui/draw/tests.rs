use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::{
    BarStyle, HorizontalBarLayout, LinearBarMode, PolygonLayout, VerticalBarLayout,
    bar_color_index, for_each_horizontal_bar_mode, for_each_polygon_bar, for_each_segment_span,
    for_each_vertical_bar_mode, radial_distribution,
};

#[test]
fn spreads_colors_evenly() {
    let indices: Vec<usize> = (0..12).map(|index| bar_color_index(index, 12, 6)).collect();
    assert_eq!(indices, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
}

#[test]
fn segments_from_start() {
    let mut spans = Vec::new();
    for_each_segment_span(10.0, 3.0, 1.0, true, |start, len| spans.push((start, len)));
    assert_eq!(spans, vec![(0.0, 3.0), (4.0, 3.0), (8.0, 2.0)]);
}

#[test]
fn segments_from_end() {
    let mut spans = Vec::new();
    for_each_segment_span(10.0, 3.0, 1.0, false, |start, len| spans.push((start, len)));
    assert_eq!(spans, vec![(7.0, 3.0), (3.0, 3.0), (0.0, 2.0)]);
}

#[test]
fn radial_partial_arc_stays_inside_requested_span() {
    let Some(distribution) = radial_distribution(5, 100.0, 8.0, 12.0, -PI, PI) else {
        panic!("expected radial distribution");
    };
    let first_center = distribution.first_angle;
    let last_center = distribution.first_angle + (4.0 * distribution.angle_step);
    let half_bar_angle = distribution.tangential_thickness * 0.5 / 100.0;

    assert!((first_center - (-PI + half_bar_angle)).abs() < 1e-6);
    assert!((last_center - (0.0 - half_bar_angle)).abs() < 1e-6);
}

#[test]
fn radial_single_bar_centers_inside_partial_arc() {
    let Some(distribution) = radial_distribution(1, 120.0, 8.0, 12.0, -FRAC_PI_2, PI) else {
        panic!("expected radial distribution");
    };
    assert!((distribution.first_angle - 0.0).abs() < 1e-6);
    assert_eq!(distribution.angle_step, 0.0);
}

#[test]
fn radial_full_circle_clamps_oversized_arc() {
    let Some(distribution) = radial_distribution(4, 100.0, 8.0, 12.0, -FRAC_PI_2, TAU * 2.0) else {
        panic!("expected radial distribution");
    };
    let expected_step = TAU / 4.0;

    assert!((distribution.first_angle - (-FRAC_PI_2)).abs() < 1e-6);
    assert!((distribution.angle_step - expected_step).abs() < 1e-6);
}

#[test]
fn polygon_layout_distributes_bars_across_multiple_edges() {
    let mut angles = Vec::new();
    for_each_polygon_bar(
        &[1.0, 1.0, 1.0],
        PolygonLayout {
            width: 800.0,
            height: 800.0,
            radius: 180.0,
            rotation_radians: -FRAC_PI_2,
            sides: 3,
        },
        BarStyle {
            thickness: 8.0,
            gap: 0.0,
            corner_radius: 0.0,
            segmented: false,
            segment_length: 12.0,
            segment_gap: 6.0,
        },
        |_, spec| angles.push((spec.angle.to_degrees() * 10.0).round() as i32),
    );

    angles.sort_unstable();
    angles.dedup();
    assert_eq!(angles.len(), 3);
}

#[test]
fn split_horizontal_mode_leaves_center_gap() {
    let mut positions = Vec::new();
    for_each_horizontal_bar_mode(
        &[1.0, 1.0, 1.0, 1.0],
        HorizontalBarLayout {
            width: 400.0,
            height: 100.0,
            bar_thickness: 20.0,
            gap: 10.0,
            from_top: false,
            mode: LinearBarMode::Split { center_gap: 80.0 },
        },
        |_, x, _, bar_width, _| positions.push((x, bar_width)),
    );

    assert_eq!(positions.len(), 4);
    let left_end = positions[1].0 + positions[1].1;
    let right_start = positions[2].0;
    assert!(right_start - left_end >= 80.0 - 1e-6);
}

#[test]
fn split_vertical_mode_leaves_center_gap() {
    let mut positions = Vec::new();
    for_each_vertical_bar_mode(
        &[1.0, 1.0, 1.0, 1.0],
        VerticalBarLayout {
            width: 100.0,
            height: 400.0,
            bar_thickness: 20.0,
            gap: 10.0,
            from_left: false,
            mode: LinearBarMode::Split { center_gap: 80.0 },
        },
        |_, _, y, _, bar_height| positions.push((y, bar_height)),
    );

    assert_eq!(positions.len(), 4);
    let top_end = positions[1].0 + positions[1].1;
    let bottom_start = positions[2].0;
    assert!(bottom_start - top_end >= 80.0 - 1e-6);
}
