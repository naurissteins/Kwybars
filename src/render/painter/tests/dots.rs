use std::time::{Duration, Instant};

use super::{FRAMES, SIZE, blank, config, paint};
use crate::config::{ColorMode, Edge, Layout, Rgba, SurfaceConfig};
use crate::render::{ByteOrder, Painter, PixelRect};

fn dots(layout: Layout, edge: Edge) -> SurfaceConfig {
    config(|c| {
        c.visualizer.layout = layout;
        c.overlay.position = edge;
        c.visualizer.bar_width = 5;
        c.visualizer.gap = 2;
        c.visualizer.color_mode = ColorMode::Gradient;
        c.visualizer.color_rgba = Rgba::new(0.2, 0.6, 1.0, 0.5);
        c.visualizer.color2_rgba = Rgba::new(1.0, 0.3, 0.1, 0.8);
    })
}

fn assert_patching_matches(surface: &SurfaceConfig, opacities: &[u8]) {
    let mut painter = Painter::new(surface, 6, SIZE, 1.0, ByteOrder::Rgba);
    let mut data = vec![0x5A_u8; (SIZE.0 * SIZE.1 * 4) as usize];
    let mut contents = painter.new_contents();
    let start = Instant::now();
    let frames = FRAMES.iter().chain(&FRAMES).chain(&[[0.0; 6]; 8]);
    for (frame, (heights, opacity)) in frames.zip(opacities.iter().cycle()).enumerate() {
        let now = start + Duration::from_millis(16 * frame as u64);
        painter.layout(heights, *opacity, now);
        paint(&painter, &mut data, &mut contents);
        let mut expected = blank();
        paint(&painter, &mut expected, &mut painter.new_contents());
        assert!(data == expected, "patched pixels differ at frame {frame}");
    }
}

#[test]
fn dot_patching_matches_a_whole_paint() {
    for layout in [Layout::Particle, Layout::Floating] {
        for edge in [Edge::Bottom, Edge::Top, Edge::Left, Edge::Right] {
            assert_patching_matches(&dots(layout, edge), &[255]);
            assert_patching_matches(&dots(layout, edge), &[255, 200, 90, 90, 0, 31]);
        }
    }
}

#[test]
fn floating_dots_animate_until_they_settle() {
    let surface = dots(Layout::Floating, Edge::Bottom);
    let mut painter = Painter::new(&surface, 6, SIZE, 1.0, ByteOrder::Rgba);
    let start = Instant::now();
    let at = |frame: u64| start + Duration::from_millis(16 * frame);
    painter.layout(&[0.0; 6], 255, at(0));
    assert!(!painter.animates());
    for frame in 1..4 {
        painter.layout(&[0.9; 6], 255, at(frame));
    }
    assert!(painter.animates());
    painter.present(|_| {});
    // bars at rest, dots still falling: every frame shows something new
    let mut frame = 4;
    while painter.animates() {
        assert!(painter.layout(&[0.0; 6], 255, at(frame)), "frame {frame}");
        painter.present(|_| {});
        frame += 1;
        assert!(frame < 200, "dots never settled");
    }
    assert!(!painter.layout(&[0.0; 6], 255, at(frame)));

    let still = dots(Layout::Particle, Edge::Bottom);
    let mut painter = Painter::new(&still, 6, SIZE, 1.0, ByteOrder::Rgba);
    painter.layout(&[0.9; 6], 255, at(0));
    painter.layout(&[0.9; 6], 255, at(1));
    assert!(!painter.animates());
}

#[test]
fn a_rising_dot_damages_only_its_own_column() {
    let surface = dots(Layout::Floating, Edge::Bottom);
    let mut painter = Painter::new(&surface, 6, SIZE, 1.0, ByteOrder::Rgba);
    let start = Instant::now();
    let mut heights = [0.3; 6];
    painter.layout(&heights, 255, start);
    painter.present(|_| {});
    heights[2] = 1.0;
    let mut areas: Vec<PixelRect> = Vec::new();
    for frame in 1..4 {
        // 0.3 pushes less than gravity pulls: only dot 2 leaves the edge
        painter.layout(&heights, 255, start + Duration::from_millis(17 * frame));
        painter.present(|area| areas.push(area));
    }
    assert!(!areas.is_empty());
    let column = areas[0].x..areas[0].right();
    assert!(
        areas
            .iter()
            .all(|area| area.x >= column.start && area.right() <= column.end)
    );
    assert!(areas.iter().any(|area| area.height > 10), "{areas:?}");
}

#[test]
fn dots_take_their_bar_colors() {
    let surface = dots(Layout::Particle, Edge::Bottom);
    let mut painter = Painter::new(&surface, 6, SIZE, 1.0, ByteOrder::Rgba);
    painter.layout(&[1.0; 6], 255, Instant::now());
    let mut data = blank();
    paint(&painter, &mut data, &mut painter.new_contents());
    let pixel = |x: u32, y: u32| {
        let at = ((y * SIZE.0 + x) * 4) as usize;
        [data[at], data[at + 1], data[at + 2], data[at + 3]]
    };
    // six dots of 10 px with 2 px gaps: 70 px centered in 120, at mid height
    let first = ByteOrder::Rgba.pack(Rgba::new(0.2, 0.6, 1.0, 0.5));
    let last = ByteOrder::Rgba.pack(Rgba::new(1.0, 0.3, 0.1, 0.8));
    assert_eq!(pixel(30, 30), first);
    assert_eq!(pixel(89, 30), last);
    assert_eq!(pixel(60, 30), [0; 4]);
}
