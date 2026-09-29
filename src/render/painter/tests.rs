use super::{BufferContents, Painter};
use crate::config::{ColorMode, Config, Edge, GradientDirection, LineMode, Rgba, SurfaceConfig};
use crate::render::{ByteOrder, Canvas, PixelRect};

const SIZE: (u32, u32) = (120, 60);

fn config(edit: impl FnOnce(&mut Config)) -> SurfaceConfig {
    let mut config = Config::default();
    config.visualizer.bars = 6;
    config.visualizer.bar_width = 12;
    config.visualizer.gap = 6;
    config.visualizer.bar_corner_radius = 0.0;
    config.visualizer.color_mode = ColorMode::Solid;
    config.visualizer.color_rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);
    edit(&mut config);
    config.surface(None, None)
}

fn paint(painter: &Painter, data: &mut [u8], contents: &mut BufferContents) {
    let Some(mut canvas) = Canvas::new(data, SIZE) else {
        panic!("canvas");
    };
    painter.paint(&mut canvas, contents);
}

fn blank() -> Vec<u8> {
    vec![0_u8; (SIZE.0 * SIZE.1 * 4) as usize]
}

/// paints `frames` into one reused buffer and compares each with a fresh paint
fn assert_patching_matches(surface: &SurfaceConfig, frames: &[[f32; 6]]) {
    let mut patched = Painter::new(surface, 6, SIZE, 1.0, ByteOrder::Rgba);
    let mut data = vec![0x5A_u8; (SIZE.0 * SIZE.1 * 4) as usize];
    let mut contents = patched.new_contents();
    for heights in frames {
        patched.layout(heights);
        paint(&patched, &mut data, &mut contents);

        let mut fresh = Painter::new(surface, 6, SIZE, 1.0, ByteOrder::Rgba);
        fresh.layout(heights);
        let mut expected = blank();
        let mut fresh_contents = fresh.new_contents();
        paint(&fresh, &mut expected, &mut fresh_contents);
        assert!(data == expected, "patched pixels differ at {heights:?}");
    }
}

const FRAMES: [[f32; 6]; 6] = [
    [0.3, 0.9, 0.0, 0.47, 1.0, 0.2],
    [0.55, 0.2, 0.61, 0.47, 0.03, 0.9],
    [0.1, 0.33, 0.9, 0.05, 0.5, 0.91],
    [0.12, 0.0, 0.88, 0.5, 0.52, 0.2],
    [0.7, 0.71, 0.1, 0.1, 0.49, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
];

#[test]
fn patching_matches_a_fresh_paint_for_plain_bars() {
    for edge in [Edge::Bottom, Edge::Top, Edge::Left, Edge::Right] {
        assert_patching_matches(&config(|c| c.overlay.position = edge), &FRAMES);
    }
}

#[test]
fn patching_matches_a_fresh_paint_for_rounded_translucent_gradient_bars() {
    for edge in [Edge::Bottom, Edge::Top, Edge::Left, Edge::Right] {
        let surface = config(|c| {
            c.overlay.position = edge;
            c.visualizer.bar_corner_radius = 20.0;
            c.visualizer.color_mode = ColorMode::Gradient;
            c.visualizer.color_rgba = Rgba::new(0.2, 0.6, 1.0, 0.5);
            c.visualizer.color2_rgba = Rgba::new(1.0, 0.3, 0.1, 0.8);
        });
        assert_patching_matches(&surface, &FRAMES);
    }
}

#[test]
fn patching_matches_a_fresh_paint_for_segmented_split_bars() {
    let edges = [Edge::Bottom, Edge::Top, Edge::Left, Edge::Right];
    let directions = [GradientDirection::Vertical, GradientDirection::Horizontal];
    for (edge, direction) in edges.into_iter().flat_map(|e| directions.map(|d| (e, d))) {
        let surface = config(|c| {
            c.overlay.position = edge;
            c.visualizer.segmented_bars = true;
            c.visualizer.segment_length = 7;
            c.visualizer.segment_gap = 3;
            c.visualizer.bar_corner_radius = 3.0;
            c.visualizer.line_mode = LineMode::Split;
            c.visualizer.line_split_gap = 20;
            c.visualizer.gradient_direction = direction;
        });
        let mut themed = surface.clone();
        themed.theme_colors = Some([Rgba::new(0.5, 0.5, 0.5, 1.0); 6]);
        assert_patching_matches(&surface, &FRAMES);
        assert_patching_matches(&themed, &FRAMES);
    }
}

#[test]
fn damage_covers_only_what_moved() {
    let mut painter = Painter::new(&config(|_| {}), 6, SIZE, 1.0, ByteOrder::Rgba);
    painter.layout(&[0.5; 6]);
    let mut areas = Vec::new();
    painter.present(|area| areas.push(area));
    assert_eq!(areas, vec![PixelRect::full(SIZE)]);

    let mut heights = [0.5; 6];
    heights[2] = 0.75;
    assert!(painter.layout(&heights));
    areas.clear();
    painter.present(|area| areas.push(area));
    assert_eq!(areas.len(), 1);
    // bar 2 grew from 30 to 45 of 60 pixels: rows 15..30 only
    assert_eq!((areas[0].y, areas[0].height), (15, 15));
    assert!(!painter.layout(&heights));
}

#[test]
fn silent_bars_keep_a_small_stub() {
    let mut painter = Painter::new(&config(|_| {}), 6, SIZE, 1.5, ByteOrder::Rgba);
    painter.layout(&[0.0; 6]);
    let mut data = blank();
    paint(&painter, &mut data, &mut painter.new_contents());
    let lit = |y: u32| (0..SIZE.0).any(|x| data[((y * SIZE.0 + x) * 4 + 3) as usize] > 0);
    // 2 logical pixels at scale 1.5 are 3 rows
    assert_eq!((0..SIZE.1).filter(|y| lit(*y)).count(), 3);
}

#[test]
fn painting_does_not_reallocate() {
    let mut painter = Painter::new(&config(|_| {}), 6, SIZE, 1.0, ByteOrder::Rgba);
    let mut data = blank();
    let mut contents = painter.new_contents();
    let before = (painter.next.as_ptr(), painter.shown.as_ptr());
    for step in 0..50 {
        painter.layout(&[step as f32 / 50.0; 6]);
        paint(&painter, &mut data, &mut contents);
        painter.present(|_| {});
    }
    assert_eq!(before, (painter.next.as_ptr(), painter.shown.as_ptr()));
}
