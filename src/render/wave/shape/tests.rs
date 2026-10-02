use super::WaveShape;
use crate::config::{Config, Edge, LineMode, SurfaceConfig};

/// a wave with a stroke of 4, so legacy's edge padding of 2 + 6 = 8
fn surface(edge: Edge, edit: impl FnOnce(&mut Config)) -> SurfaceConfig {
    let mut config = Config::default();
    config.overlay.position = edge;
    config.visualizer.wave_stroke_width = 4;
    config.visualizer.wave_glow = false;
    config.visualizer.wave_amplitude = 1.0;
    edit(&mut config);
    config.surface(None, None)
}

fn points(surface: &SurfaceConfig, size: (u32, u32), values: &[f32]) -> Vec<(f32, f32)> {
    let shape = WaveShape::new(surface, size, 1.0, values.len());
    let mut out = Vec::new();
    shape.points(values, &mut out);
    out
}

// legacy `horizontal_wave_points_center_flat_input`
#[test]
fn equal_values_are_a_flat_line_in_the_middle() {
    let out = points(&surface(Edge::Bottom, |_| {}), (300, 100), &[0.4, 0.4, 0.4]);
    assert_eq!(out.len(), 3);
    assert!(out.iter().all(|point| (point.1 - 50.0).abs() < 1e-4));
    assert_eq!((out[0].0, out[2].0), (8.0, 292.0));
}

// legacy `horizontal_wave_split_preserves_center_gap`
#[test]
fn split_mode_leaves_the_center_gap() {
    let split = surface(Edge::Bottom, |c| {
        c.visualizer.line_mode = LineMode::Split;
        c.visualizer.line_split_gap = 80;
    });
    let out = points(&split, (400, 100), &[0.1, 0.9, 0.3, 0.7]);
    assert_eq!(out.len(), 4);
    assert!(out[2].0 - out[1].0 >= 80.0 - 1e-4);
}

// legacy `vertical_wave_points_span_width_from_relative_levels`
#[test]
fn a_side_edge_swings_across_the_width() {
    let out = points(&surface(Edge::Right, |_| {}), (100, 200), &[0.0, 1.0]);
    assert!(out[0].0 > out[1].0);
    assert!(out[0].0 < 98.0 && out[1].0 > 2.0);
    assert!(out[0].0 > 50.0 && out[1].0 < 50.0);
    assert_eq!((out[0].1, out[1].1), (8.0, 192.0));
}

// legacy `wave_smoothing_scale_is_clamped`
#[test]
fn smoothing_is_clamped_into_the_control_scale() {
    let control = |smoothing: f32| {
        let surface = surface(Edge::Bottom, |c| c.visualizer.wave_smoothing = smoothing);
        WaveShape::new(&surface, (300, 100), 1.0, 3).control
    };
    assert_eq!(control(0.0), 0.0);
    assert!((control(1.0) - 1.0 / 6.0).abs() < 1e-6);
    assert!((control(3.0) - 2.0 / 6.0).abs() < 1e-6);
}

// legacy `quiet_wave_variation_stays_close_to_center`
#[test]
fn quiet_differences_stay_near_the_middle() {
    let out = points(
        &surface(Edge::Bottom, |_| {}),
        (300, 100),
        &[0.10, 0.12, 0.11],
    );
    assert!(out.iter().all(|point| (point.1 - 50.0).abs() < 4.0));
}

// legacy `wave_amplitude_scales_height_without_reintroducing_full_range_expansion`
#[test]
fn amplitude_scales_the_swing() {
    let at = |amplitude: f32| {
        let surface = surface(Edge::Bottom, |c| c.visualizer.wave_amplitude = amplitude);
        points(&surface, (200, 100), &[0.0, 1.0])
    };
    let (low, high) = (at(0.5), at(1.5));
    assert!(high[0].1 > low[0].1);
    assert!(high[1].1 < low[1].1);
}

// legacy `wave_from_start_inverts_horizontal_direction`
#[test]
fn a_top_edge_swings_the_other_way() {
    let bottom = points(&surface(Edge::Bottom, |_| {}), (200, 100), &[0.0, 1.0]);
    let top = points(&surface(Edge::Top, |_| {}), (200, 100), &[0.0, 1.0]);
    assert!(top[0].1 < bottom[0].1);
    assert!(top[1].1 > bottom[1].1);
}

// legacy `wave_edge_padding_keeps_strong_peaks_off_the_border`
#[test]
fn padding_keeps_peaks_off_the_border() {
    let out = points(&surface(Edge::Bottom, |_| {}), (200, 100), &[0.0, 1.0]);
    assert!(out[0].1 <= 92.0);
    assert!(out[1].1 >= 8.0);
}

#[test]
fn sizes_scale_and_the_glow_widens_the_padding() {
    let glowing = surface(Edge::Bottom, |c| {
        c.visualizer.wave_glow = true;
        c.visualizer.wave_fill = true;
    });
    let shape = WaveShape::new(&glowing, (300, 150), 1.5, 3);
    // stroke 4, glow 12: padding 6 + 6 = 12 logical
    assert_eq!((shape.stroke, shape.glow), (6.0, Some(18.0)));
    assert_eq!(shape.baseline, Some(150.0 - 18.0));
    let top = WaveShape::new(
        &surface(Edge::Top, |c| c.visualizer.wave_fill = true),
        (300, 150),
        1.0,
        3,
    );
    assert_eq!(top.baseline, Some(8.0));
    let plain = WaveShape::new(
        &surface(Edge::Top, |c| c.visualizer.wave_fill = false),
        (300, 150),
        1.0,
        3,
    );
    assert_eq!((plain.glow, plain.baseline), (None, None));
}
