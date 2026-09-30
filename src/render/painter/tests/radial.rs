use std::time::{Duration, Instant};

use super::{SIZE, assert_timed_patching_matches, blank, config, paint};
use crate::config::{ColorMode, Config, GradientDirection, Layout, Rgba, SurfaceConfig};
use crate::render::{ByteOrder, Painter, PixelRect};

fn radial(speed: f32, edit: impl FnOnce(&mut Config)) -> SurfaceConfig {
    config(|c| {
        c.visualizer.layout = Layout::Radial;
        c.visualizer.bar_width = 3;
        c.visualizer.gap = 2;
        c.visualizer.radial_inner_radius = 10;
        c.visualizer.radial_start_angle = 0.0;
        c.visualizer.radial_rotation_speed = speed;
        c.visualizer.color_mode = ColorMode::Gradient;
        c.visualizer.color_rgba = Rgba::new(0.2, 0.6, 1.0, 0.5);
        c.visualizer.color2_rgba = Rgba::new(1.0, 0.3, 0.1, 0.8);
        edit(c);
    })
}

#[test]
fn radial_patching_matches_a_whole_paint() {
    let variants: [&dyn Fn(&mut Config); 5] = [
        &|_| {},
        &|c| c.visualizer.bar_corner_radius = 2.0,
        &|c| {
            c.visualizer.segmented_bars = true;
            c.visualizer.segment_length = 3;
            c.visualizer.segment_gap = 1;
        },
        &|c| {
            c.visualizer.radial_start_angle = -90.0;
            c.visualizer.radial_arc_degrees = 180.0;
            c.visualizer.center_offset_x = 30.0;
        },
        &|c| c.visualizer.gradient_direction = GradientDirection::Horizontal,
    ];
    for speed in [0.0, 90.0, -400.0] {
        for edit in variants {
            let mut surface = radial(speed, edit);
            assert_timed_patching_matches(&surface, &[255]);
            assert_timed_patching_matches(&surface, &[255, 200, 90, 90, 0, 31]);
            surface.theme_colors = Some([Rgba::new(0.5, 0.5, 0.5, 1.0); 6]);
            assert_timed_patching_matches(&surface, &[255]);
        }
    }
}

#[test]
fn a_turning_ring_animates_and_damages_each_bar() {
    let mut painter = Painter::new(&radial(90.0, |_| {}), 6, SIZE, 1.0, ByteOrder::Rgba);
    let start = Instant::now();
    painter.layout(&[0.5; 6], 255, start);
    assert!(painter.animates());
    painter.present(|_| {});
    // the bars at rest still turn
    assert!(painter.layout(&[0.5; 6], 255, start + Duration::from_millis(16)));
    let mut areas = Vec::new();
    painter.present(|area| areas.push(area));
    // each bar where it was and is, around the center
    assert_eq!(areas.len(), 6, "{areas:?}");
    assert!(
        areas.iter().all(|area| area.x > 20 && area.right() < 100),
        "{areas:?}"
    );

    let still = Painter::new(&radial(0.0, |_| {}), 6, SIZE, 1.0, ByteOrder::Rgba);
    assert!(!still.animates());
}

#[test]
fn a_still_ring_redraws_around_one_growing_bar() {
    let mut painter = Painter::new(&radial(0.0, |_| {}), 6, SIZE, 1.0, ByteOrder::Rgba);
    let start = Instant::now();
    let mut heights = [0.3; 6];
    painter.layout(&heights, 255, start);
    painter.present(|_| {});
    heights[0] = 0.9;
    assert!(painter.layout(&heights, 255, start + Duration::from_millis(16)));
    let mut areas: Vec<PixelRect> = Vec::new();
    painter.present(|area| areas.push(area));
    // bar 0 points right from the center at 60, 30
    assert_eq!(areas.len(), 1);
    assert!(areas[0].x >= 60 && areas[0].height < 10, "{areas:?}");
    assert!(!painter.layout(&heights, 255, start + Duration::from_millis(32)));
}

#[test]
fn patching_a_few_bars_redraws_their_neighbors() {
    let surface = radial(0.0, |c| {
        c.visualizer.bars = 24;
        c.visualizer.bar_corner_radius = 1.5;
    });
    let mut painter = Painter::new(&surface, 24, SIZE, 1.0, ByteOrder::Rgba);
    let mut data = blank();
    let mut contents = painter.new_contents();
    let start = Instant::now();
    let mut heights = [0.6_f32; 24];
    for frame in 0..30_u64 {
        // two bars at a time move, the rest stay
        let bar = (frame as usize * 5) % 24;
        heights[bar] = (frame as f32 * 0.37).sin().abs();
        heights[(bar + 1) % 24] = (frame as f32 * 0.61).cos().abs();
        painter.layout(&heights, 255, start + Duration::from_millis(16 * frame));
        paint(&painter, &mut data, &mut contents);
        let mut expected = blank();
        paint(&painter, &mut expected, &mut painter.new_contents());
        assert!(data == expected, "patched pixels differ at frame {frame}");
    }
}
