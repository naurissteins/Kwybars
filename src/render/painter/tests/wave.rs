use std::time::{Duration, Instant};

use super::{SIZE, assert_timed_patching_matches, config};
use crate::config::{ColorMode, Config, Edge, Layout, LineMode, Rgba, SurfaceConfig};
use crate::render::{ByteOrder, Painter, PixelRect};

fn wave(edge: Edge, edit: impl FnOnce(&mut Config)) -> SurfaceConfig {
    config(|c| {
        c.visualizer.layout = Layout::Wave;
        c.overlay.position = edge;
        c.visualizer.wave_stroke_width = 3;
        c.visualizer.wave_motion_smoothing = 0.4;
        c.visualizer.color_mode = ColorMode::Gradient;
        c.visualizer.color_rgba = Rgba::new(0.2, 0.6, 1.0, 0.5);
        c.visualizer.color2_rgba = Rgba::new(1.0, 0.3, 0.1, 0.8);
        edit(c);
    })
}

#[test]
fn wave_patching_matches_a_whole_paint() {
    let variants: [&dyn Fn(&mut Config); 4] = [
        &|c| c.visualizer.wave_fill = false,
        &|c| c.visualizer.wave_fill = true,
        &|c| {
            c.visualizer.wave_glow = true;
            c.visualizer.wave_smoothing = 2.0;
        },
        &|c| {
            c.visualizer.wave_amplitude = 2.0;
            c.visualizer.line_mode = LineMode::Split;
            c.visualizer.line_split_gap = 30;
        },
    ];
    for edge in [Edge::Bottom, Edge::Top, Edge::Left, Edge::Right] {
        for edit in variants {
            let mut surface = wave(edge, edit);
            assert_timed_patching_matches(&surface, &[255]);
            assert_timed_patching_matches(&surface, &[255, 200, 90, 90, 0, 31]);
            surface.theme_colors = Some([Rgba::new(0.5, 0.5, 0.5, 1.0); 6]);
            assert_timed_patching_matches(&surface, &[255]);
        }
    }
}

#[test]
fn a_wave_animates_until_it_arrives_and_damages_where_it_was_and_is() {
    let mut painter = Painter::new(&wave(Edge::Bottom, |_| {}), 6, SIZE, 1.0, ByteOrder::Rgba);
    let start = Instant::now();
    let at = |frame: u64| start + Duration::from_millis(16 * frame);
    painter.layout(&[0.0; 6], 255, at(0));
    assert!(!painter.animates());
    let mut areas: Vec<PixelRect> = Vec::new();
    painter.present(|area| areas.push(area));
    assert_eq!(areas, vec![PixelRect::full(SIZE)]);

    assert!(painter.layout(&[0.0, 1.0, 0.0, 0.0, 0.2, 0.0], 255, at(1)));
    assert!(painter.animates());
    areas.clear();
    painter.present(|area| areas.push(area));
    // one area holding the flat line and the new curve
    assert_eq!(areas.len(), 1);
    assert!(areas[0].height > 5 && areas[0].height < SIZE.1, "{areas:?}");

    // the bars at rest: the wave still eases towards them, then stops
    let mut frame = 2;
    while painter.animates() && frame < 2_000 {
        assert!(painter.layout(&[0.0; 6], 255, at(frame)), "frame {frame}");
        painter.present(|_| {});
        frame += 1;
    }
    assert!(!painter.animates() && frame > 5);
    assert!(!painter.layout(&[0.0; 6], 255, at(frame)));
}
