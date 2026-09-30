mod cost;
mod dots;
mod radial;

use std::time::{Duration, Instant};

use super::{BufferContents, Painter};
use crate::config::{
    ColorMode, Config, Edge, FrameMirrorMode, GradientDirection, Layout, LineMode,
    MirrorOrientation, Rgba, SurfaceConfig,
};
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
    assert_fading_patching_matches(surface, frames, &[255]);
}

/// like [`assert_patching_matches`], cycling through `opacities`
fn assert_fading_patching_matches(surface: &SurfaceConfig, frames: &[[f32; 6]], opacities: &[u8]) {
    let mut patched = Painter::new(surface, 6, SIZE, 1.0, ByteOrder::Rgba);
    let mut data = vec![0x5A_u8; (SIZE.0 * SIZE.1 * 4) as usize];
    let mut contents = patched.new_contents();
    for (heights, opacity) in frames.iter().zip(opacities.iter().cycle()) {
        patched.layout(heights, *opacity, Instant::now());
        paint(&patched, &mut data, &mut contents);

        let mut fresh = Painter::new(surface, 6, SIZE, 1.0, ByteOrder::Rgba);
        fresh.layout(heights, *opacity, Instant::now());
        let mut expected = blank();
        let mut fresh_contents = fresh.new_contents();
        paint(&fresh, &mut expected, &mut fresh_contents);
        assert!(
            data == expected,
            "patched pixels differ at {heights:?}, opacity {opacity}"
        );
    }
}

/// frames 16 ms apart, compared with a whole paint of the same layout
pub(super) fn assert_timed_patching_matches(surface: &SurfaceConfig, opacities: &[u8]) {
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
fn patching_matches_a_fresh_paint_while_fading() {
    let opacities = [255, 200, 200, 90, 0, 31, 31];
    let rounded = config(|c| {
        c.visualizer.bar_corner_radius = 20.0;
        c.visualizer.color_mode = ColorMode::Gradient;
        c.visualizer.color_rgba = Rgba::new(0.2, 0.6, 1.0, 0.5);
        c.visualizer.color2_rgba = Rgba::new(1.0, 0.3, 0.1, 0.8);
    });
    let segmented = config(|c| {
        c.overlay.position = Edge::Left;
        c.visualizer.segmented_bars = true;
        c.visualizer.segment_length = 7;
        c.visualizer.segment_gap = 3;
    });
    for surface in [rounded, segmented] {
        assert_fading_patching_matches(&surface, &FRAMES, &opacities);
        assert_fading_patching_matches(&surface, &[FRAMES[0]; 4], &opacities);
    }
}

#[test]
fn a_new_opacity_damages_every_bar_whole() {
    let mut painter = Painter::new(&config(|_| {}), 6, SIZE, 1.0, ByteOrder::Rgba);
    painter.layout(&[0.5; 6], 255, Instant::now());
    painter.present(|_| {});
    assert!(painter.layout(&[0.5; 6], 128, Instant::now()));
    let mut areas = Vec::new();
    painter.present(|area| areas.push(area));
    assert_eq!(areas.len(), 6);
    assert!(areas.iter().all(|area| (area.y, area.height) == (30, 30)));
    assert!(!painter.layout(&[0.5; 6], 128, Instant::now()));
}

#[test]
fn damage_covers_only_what_moved() {
    let mut painter = Painter::new(&config(|_| {}), 6, SIZE, 1.0, ByteOrder::Rgba);
    painter.layout(&[0.5; 6], 255, Instant::now());
    let mut areas = Vec::new();
    painter.present(|area| areas.push(area));
    assert_eq!(areas, vec![PixelRect::full(SIZE)]);

    let mut heights = [0.5; 6];
    heights[2] = 0.75;
    assert!(painter.layout(&heights, 255, Instant::now()));
    areas.clear();
    painter.present(|area| areas.push(area));
    assert_eq!(areas.len(), 1);
    // bar 2 grew from 30 to 45 of 60 pixels: rows 15..30 only
    assert_eq!((areas[0].y, areas[0].height), (15, 15));
    assert!(!painter.layout(&heights, 255, Instant::now()));
}

#[test]
fn silent_bars_keep_a_small_stub() {
    let mut painter = Painter::new(&config(|_| {}), 6, SIZE, 1.5, ByteOrder::Rgba);
    painter.layout(&[0.0; 6], 255, Instant::now());
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
        painter.layout(&[step as f32 / 50.0; 6], (step * 5) as u8, Instant::now());
        paint(&painter, &mut data, &mut contents);
        painter.present(|_| {});
    }
    assert_eq!(before, (painter.next.as_ptr(), painter.shown.as_ptr()));
}

fn mirror(orientation: MirrorOrientation, edit: impl FnOnce(&mut Config)) -> SurfaceConfig {
    config(|c| {
        c.visualizer.layout = Layout::Mirror;
        c.visualizer.mirror_orientation = orientation;
        edit(c);
    })
}

#[test]
fn mirror_patching_matches_a_fresh_paint() {
    let opacities = [255, 255, 180, 180, 40, 255];
    for orientation in [MirrorOrientation::Horizontal, MirrorOrientation::Vertical] {
        let rounded = mirror(orientation, |c| {
            c.visualizer.mirror_gap = 6;
            c.visualizer.bar_corner_radius = 20.0;
            c.visualizer.color_mode = ColorMode::Gradient;
            c.visualizer.color_rgba = Rgba::new(0.2, 0.6, 1.0, 0.5);
            c.visualizer.color2_rgba = Rgba::new(1.0, 0.3, 0.1, 0.8);
        });
        let segmented = mirror(orientation, |c| {
            c.visualizer.segmented_bars = true;
            c.visualizer.segment_length = 5;
            c.visualizer.segment_gap = 2;
            c.visualizer.line_mode = LineMode::Split;
            c.visualizer.line_split_gap = 10;
        });
        assert_fading_patching_matches(&rounded, &FRAMES, &opacities);
        assert_patching_matches(&segmented, &FRAMES);
    }
}

#[test]
fn mirror_halves_are_mirror_images() {
    for (orientation, gap) in [
        (MirrorOrientation::Horizontal, 0),
        (MirrorOrientation::Horizontal, 7),
        (MirrorOrientation::Vertical, 4),
    ] {
        let surface = mirror(orientation, |c| {
            c.visualizer.mirror_gap = gap;
            c.visualizer.segmented_bars = true;
            c.visualizer.segment_length = 4;
            c.visualizer.segment_gap = 2;
            c.visualizer.bar_corner_radius = 1.0;
        });
        let mut painter = Painter::new(&surface, 6, SIZE, 1.0, ByteOrder::Rgba);
        painter.layout(&[0.3, 0.9, 0.05, 0.47, 1.0, 0.2], 255, Instant::now());
        let mut data = blank();
        paint(&painter, &mut data, &mut painter.new_contents());
        let alpha = |x: u32, y: u32| data[((y * SIZE.0 + x) * 4 + 3) as usize];
        let (width, height) = SIZE;
        for y in 0..height {
            for x in 0..width {
                let mirrored = match orientation {
                    MirrorOrientation::Horizontal => alpha(x, height - 1 - y),
                    MirrorOrientation::Vertical => alpha(width - 1 - x, y),
                };
                // edge coverage may round the other way on one side
                let difference = alpha(x, y).abs_diff(mirrored);
                assert!(difference <= 1, "{orientation:?} gap {gap} at {x},{y}");
            }
        }
        // something was drawn, and the gap stays empty
        assert!(data.iter().any(|byte| *byte != 0));
        if orientation == MirrorOrientation::Horizontal && gap == 7 {
            assert!((0..width).all(|x| alpha(x, height / 2) == 0));
        }
    }
}

fn frame(mode: FrameMirrorMode, edit: impl FnOnce(&mut Config)) -> SurfaceConfig {
    config(|c| {
        c.visualizer.layout = Layout::Frame;
        c.visualizer.frame_edges = vec![Edge::Top, Edge::Right, Edge::Bottom, Edge::Left];
        c.visualizer.frame_mirror_mode = mode;
        c.visualizer.bar_width = 4;
        c.visualizer.gap = 2;
        c.overlay.anchor_margin = 1;
        c.overlay.margin_left = 2;
        c.overlay.margin_right = 2;
        c.overlay.margin_top = 1;
        c.overlay.margin_bottom = 1;
        c.overlay.height = 14;
        c.overlay.width = 16;
        edit(c);
    })
}

#[test]
fn frame_patching_matches_a_fresh_paint() {
    let opacities = [255, 200, 200, 90, 255, 255];
    for mode in [
        FrameMirrorMode::Off,
        FrameMirrorMode::All,
        FrameMirrorMode::Pairs,
    ] {
        let rounded = frame(mode, |c| {
            c.visualizer.bar_corner_radius = 3.0;
            c.visualizer.color_mode = ColorMode::Gradient;
            c.visualizer.color_rgba = Rgba::new(0.2, 0.6, 1.0, 0.5);
            c.visualizer.color2_rgba = Rgba::new(1.0, 0.3, 0.1, 0.8);
        });
        let mut themed = frame(mode, |c| {
            c.visualizer.segmented_bars = true;
            c.visualizer.segment_length = 3;
            c.visualizer.segment_gap = 1;
            c.visualizer.gradient_direction = GradientDirection::Vertical;
        });
        themed.theme_colors = Some([
            Rgba::new(1.0, 0.0, 0.0, 1.0),
            Rgba::new(0.0, 1.0, 0.0, 1.0),
            Rgba::new(0.0, 0.0, 1.0, 1.0),
            Rgba::new(1.0, 1.0, 0.0, 1.0),
            Rgba::new(0.0, 1.0, 1.0, 1.0),
            Rgba::new(1.0, 0.0, 1.0, 1.0),
        ]);
        assert_fading_patching_matches(&rounded, &FRAMES, &opacities);
        assert_patching_matches(&themed, &FRAMES);
    }
}

#[test]
fn frame_palette_follows_the_value_each_edge_shows() {
    // six bars over four edges: top shows 0..1, right 1..3, bottom 3..4,
    // left 4..6, and a theme gives bar n palette color n
    let colors = [
        Rgba::new(1.0, 0.0, 0.0, 1.0),
        Rgba::new(0.0, 1.0, 0.0, 1.0),
        Rgba::new(0.0, 0.0, 1.0, 1.0),
        Rgba::new(1.0, 1.0, 0.0, 1.0),
        Rgba::new(0.0, 1.0, 1.0, 1.0),
        Rgba::new(1.0, 0.0, 1.0, 1.0),
    ];
    let mut surface = frame(FrameMirrorMode::Off, |c| {
        c.visualizer.gradient_direction = GradientDirection::Vertical;
        c.visualizer.bar_corner_radius = 0.0;
    });
    surface.theme_colors = Some(colors);
    let mut painter = Painter::new(&surface, 6, SIZE, 1.0, ByteOrder::Rgba);
    painter.layout(&[1.0; 6], 255, Instant::now());
    let mut data = blank();
    paint(&painter, &mut data, &mut painter.new_contents());
    let pixel = |x: u32, y: u32| {
        let at = ((y * SIZE.0 + x) * 4) as usize;
        [data[at], data[at + 1], data[at + 2]]
    };
    // the one bar of the top and of the bottom edge is centered at x = 60,
    // clear of the side edges
    assert_eq!(pixel(60, 5), [255, 0, 0]);
    assert_eq!(pixel(60, SIZE.1 - 5), [255, 255, 0]);
    // the right edge shows bars 1 and 2, green and blue
    let right: Vec<[u8; 3]> = (0..SIZE.1)
        .map(|y| pixel(SIZE.0 - 5, y))
        .filter(|color| *color != [0, 0, 0])
        .collect();
    assert!(right.contains(&[0, 255, 0]) && right.contains(&[0, 0, 255]));
    assert!(!right.contains(&[255, 0, 0]));
}
