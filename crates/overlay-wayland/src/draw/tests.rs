
use kwybars_common::config::{OverlayPosition, RgbaColor, VisualizerColorMode, VisualizerConfig};
use kwybars_common::spectrum::SpectrumFrame;

use super::{BarPaint, render_bars};

#[test]
fn bars_leave_transparent_background() {
    let width = 320;
    let height = 96;
    let mut canvas = vec![0xAA; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![0.25; 20], 0);

    render_bars(
        &mut canvas,
        width,
        height,
        &frame,
        &OverlayPosition::Bottom,
        &default_paint(),
    );

    assert_eq!(&canvas[0..4], &[0, 0, 0, 0]);
    assert!(canvas.chunks_exact(4).any(|pixel| pixel[3] != 0));
}

#[test]
fn bars_handle_small_buffers() {
    let mut canvas = vec![0; 4 * 8 * 8];
    let frame = SpectrumFrame::new(vec![0.25; 8], 0);
    render_bars(
        &mut canvas,
        8,
        8,
        &frame,
        &OverlayPosition::Bottom,
        &default_paint(),
    );
    assert_eq!(canvas.len(), 256);
}

#[test]
fn different_frame_values_change_output() {
    let width = 320;
    let height = 96;
    let mut first = vec![0; (width * height * 4) as usize];
    let mut second = vec![0; (width * height * 4) as usize];
    let low = SpectrumFrame::new(vec![0.15; 20], 0);
    let high = SpectrumFrame::new(vec![0.85; 20], 16);

    render_bars(
        &mut first,
        width,
        height,
        &low,
        &OverlayPosition::Bottom,
        &default_paint(),
    );
    render_bars(
        &mut second,
        width,
        height,
        &high,
        &OverlayPosition::Bottom,
        &default_paint(),
    );

    assert_ne!(first, second);
}

#[test]
fn top_position_draws_near_top_edge() {
    let width = 320;
    let height = 96;
    let mut canvas = vec![0; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![0.9; 20], 0);

    render_bars(
        &mut canvas,
        width,
        height,
        &frame,
        &OverlayPosition::Top,
        &default_paint(),
    );

    assert!(row_has_opaque_pixels(&canvas, width, 12));
    assert!(!row_has_opaque_pixels(&canvas, width, height - 4));
}

#[test]
fn right_position_draws_near_right_edge() {
    let width = 96;
    let height = 320;
    let mut canvas = vec![0; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![0.9; 20], 0);

    render_bars(
        &mut canvas,
        width,
        height,
        &frame,
        &OverlayPosition::Right,
        &default_paint(),
    );

    assert!(column_has_opaque_pixels(&canvas, width, width - 25));
    assert!(!column_has_opaque_pixels(&canvas, width, 4));
}

#[test]
fn gradient_paint_changes_bar_colors() {
    let width = 420;
    let height = 96;
    let mut canvas = vec![0; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![0.8; 8], 0);
    let paint = BarPaint::from_visualizer(
        &VisualizerConfig {
            color_mode: VisualizerColorMode::Gradient,
            color_rgba: RgbaColor {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            color2_rgba: RgbaColor {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
            ..VisualizerConfig::default()
        },
        None,
    );

    render_bars(
        &mut canvas,
        width,
        height,
        &frame,
        &OverlayPosition::Bottom,
        &paint,
    );

    let opaque_colors: std::collections::HashSet<[u8; 4]> = canvas
        .chunks_exact(4)
        .filter(|pixel| pixel[3] != 0)
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .collect();
    assert!(opaque_colors.len() > 1);
}

#[test]
fn theme_paint_distributes_palette_across_bars() {
    let width = 420;
    let height = 96;
    let mut canvas = vec![0; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![0.8; 12], 0);
    let paint = BarPaint::from_visualizer(
        &VisualizerConfig::default(),
        Some(vec![
            RgbaColor {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            RgbaColor {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            },
            RgbaColor {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
        ]),
    );

    render_bars(
        &mut canvas,
        width,
        height,
        &frame,
        &OverlayPosition::Bottom,
        &paint,
    );

    let opaque_colors: std::collections::HashSet<[u8; 4]> = canvas
        .chunks_exact(4)
        .filter(|pixel| pixel[3] != 0)
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .collect();
    assert!(opaque_colors.len() >= 3);
}

fn row_has_opaque_pixels(canvas: &[u8], width: u32, row: u32) -> bool {
    let start = (row * width * 4) as usize;
    let end = start + (width * 4) as usize;
    canvas[start..end]
        .chunks_exact(4)
        .any(|pixel| pixel[3] != 0)
}

fn column_has_opaque_pixels(canvas: &[u8], width: u32, column: u32) -> bool {
    canvas
        .chunks_exact((width * 4) as usize)
        .any(|row| row[(column * 4) as usize + 3] != 0)
}

fn default_paint() -> BarPaint {
    BarPaint::from_visualizer(&VisualizerConfig::default(), None)
}
