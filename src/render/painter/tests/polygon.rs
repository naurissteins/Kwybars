use std::time::{Duration, Instant};

use super::{SIZE, assert_timed_patching_matches, config};
use crate::config::{ColorMode, Config, Layout, Rgba, SurfaceConfig};
use crate::render::{ByteOrder, Painter};

fn polygon(sides: u32, speed: f32, edit: impl FnOnce(&mut Config)) -> SurfaceConfig {
    config(|c| {
        c.visualizer.layout = Layout::Polygon;
        c.visualizer.bar_width = 3;
        c.visualizer.gap = 2;
        c.visualizer.polygon_sides = sides;
        c.visualizer.polygon_radius = 12;
        c.visualizer.polygon_rotation_speed = speed;
        c.visualizer.color_mode = ColorMode::Gradient;
        c.visualizer.color_rgba = Rgba::new(0.2, 0.6, 1.0, 0.5);
        c.visualizer.color2_rgba = Rgba::new(1.0, 0.3, 0.1, 0.8);
        edit(c);
    })
}

#[test]
fn polygon_patching_matches_a_whole_paint() {
    let variants: [&dyn Fn(&mut Config); 3] = [
        &|_| {},
        &|c| {
            c.visualizer.bar_corner_radius = 2.0;
            c.visualizer.polygon_bar_length = 9;
        },
        &|c| {
            c.visualizer.segmented_bars = true;
            c.visualizer.segment_length = 3;
            c.visualizer.segment_gap = 1;
            c.visualizer.center_offset_x = -20.0;
        },
    ];
    for sides in [3, 4, 6] {
        for speed in [0.0, 120.0] {
            for edit in variants {
                let surface = polygon(sides, speed, edit);
                assert_timed_patching_matches(&surface, &[255]);
                assert_timed_patching_matches(&surface, &[255, 200, 90, 90, 0, 31]);
            }
        }
    }
}

#[test]
fn a_polygon_animates_only_while_it_turns() {
    let start = Instant::now();
    let mut turning = Painter::new(&polygon(3, 45.0, |_| {}), 6, SIZE, 1.0, ByteOrder::Rgba);
    turning.layout(&[0.5; 6], 255, start);
    turning.present(|_| {});
    assert!(turning.animates());
    assert!(turning.layout(&[0.5; 6], 255, start + Duration::from_millis(16)));

    let mut still = Painter::new(&polygon(3, 0.0, |_| {}), 6, SIZE, 1.0, ByteOrder::Rgba);
    still.layout(&[0.5; 6], 255, start);
    still.present(|_| {});
    assert!(!still.animates());
    assert!(!still.layout(&[0.5; 6], 255, start + Duration::from_millis(16)));
}
