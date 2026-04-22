use kwybars_common::config::{OverlayPosition, RgbaColor, VisualizerColorMode, VisualizerConfig};
use kwybars_common::spectrum::SpectrumFrame;

use super::{BarGeometry, BarPaint, RenderTarget, render_bars};

#[test]
fn bars_leave_transparent_background() {
    let width = 320;
    let height = 96;
    let mut canvas = vec![0xAA; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![0.25; 20], 0);

    render_bars(
        &mut canvas,
        RenderTarget::new(width, height, 1),
        &frame,
        &OverlayPosition::Bottom,
        &default_paint(),
        &default_geometry(),
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
        RenderTarget::new(8, 8, 1),
        &frame,
        &OverlayPosition::Bottom,
        &default_paint(),
        &default_geometry(),
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
        RenderTarget::new(width, height, 1),
        &low,
        &OverlayPosition::Bottom,
        &default_paint(),
        &default_geometry(),
    );
    render_bars(
        &mut second,
        RenderTarget::new(width, height, 1),
        &high,
        &OverlayPosition::Bottom,
        &default_paint(),
        &default_geometry(),
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
        RenderTarget::new(width, height, 1),
        &frame,
        &OverlayPosition::Top,
        &default_paint(),
        &default_geometry(),
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
        RenderTarget::new(width, height, 1),
        &frame,
        &OverlayPosition::Right,
        &default_paint(),
        &square_geometry(),
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
        RenderTarget::new(width, height, 1),
        &frame,
        &OverlayPosition::Bottom,
        &paint,
        &default_geometry(),
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
        RenderTarget::new(width, height, 1),
        &frame,
        &OverlayPosition::Bottom,
        &paint,
        &default_geometry(),
    );

    let opaque_colors: std::collections::HashSet<[u8; 4]> = canvas
        .chunks_exact(4)
        .filter(|pixel| pixel[3] != 0)
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .collect();
    assert!(opaque_colors.len() >= 3);
}

#[test]
fn horizontal_bars_use_configured_width_and_gap() {
    let width = 220;
    let height = 96;
    let mut canvas = vec![0; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![0.8; 3], 0);
    let geometry = BarGeometry::from_visualizer(&VisualizerConfig {
        bar_width: 12,
        gap: 7,
        bar_corner_radius: 0.0,
        ..VisualizerConfig::default()
    });

    render_bars(
        &mut canvas,
        RenderTarget::new(width, height, 1),
        &frame,
        &OverlayPosition::Bottom,
        &default_paint(),
        &geometry,
    );

    let runs = opaque_runs_in_row(&canvas, width, height - 16);
    assert_eq!(runs.len(), 3);
    assert!(runs.iter().all(|run| *run == 12));
}

#[test]
fn vertical_bars_use_configured_thickness_and_gap() {
    let width = 96;
    let height = 220;
    let mut canvas = vec![0; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![0.8; 3], 0);
    let geometry = BarGeometry::from_visualizer(&VisualizerConfig {
        bar_width: 14,
        gap: 9,
        ..VisualizerConfig::default()
    });

    render_bars(
        &mut canvas,
        RenderTarget::new(width, height, 1),
        &frame,
        &OverlayPosition::Right,
        &default_paint(),
        &geometry,
    );

    let runs = opaque_runs_in_column(&canvas, width, width - 30);
    assert_eq!(runs.len(), 3);
    assert!(runs.iter().all(|run| *run == 14));
}

#[test]
fn rounded_bars_trim_corner_rows() {
    let width = 160;
    let height = 110;
    let mut canvas = vec![0; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![1.0], 0);
    let geometry = BarGeometry::from_visualizer(&VisualizerConfig {
        bar_width: 40,
        bar_corner_radius: 12.0,
        ..VisualizerConfig::default()
    });

    render_bars(
        &mut canvas,
        RenderTarget::new(width, height, 1),
        &frame,
        &OverlayPosition::Bottom,
        &default_paint(),
        &geometry,
    );

    let top_row = first_opaque_row(&canvas, width, height)
        .unwrap_or_else(|| panic!("expected rounded bar pixels"));
    let top_runs = opaque_runs_in_row(&canvas, width, top_row);
    let middle_runs = opaque_runs_in_row(&canvas, width, top_row + 16);

    assert_eq!(top_runs.len(), 1);
    assert_eq!(middle_runs.len(), 1);
    assert!(top_runs[0] < middle_runs[0]);
    assert_eq!(middle_runs[0], 40);
}

#[test]
fn rounded_bars_use_partial_alpha_edges() {
    let width = 160;
    let height = 110;
    let mut canvas = vec![0; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![1.0], 0);
    let geometry = BarGeometry::from_visualizer(&VisualizerConfig {
        bar_width: 40,
        bar_corner_radius: 12.0,
        ..VisualizerConfig::default()
    });

    render_bars(
        &mut canvas,
        RenderTarget::new(width, height, 1),
        &frame,
        &OverlayPosition::Bottom,
        &default_paint(),
        &geometry,
    );

    let alphas: Vec<u8> = canvas.chunks_exact(4).map(|pixel| pixel[3]).collect();
    let max_alpha = alphas.iter().copied().max().unwrap_or(0);

    assert!(max_alpha > 0);
    assert!(alphas.iter().any(|alpha| *alpha > 0 && *alpha < max_alpha));
}

#[test]
fn zero_corner_radius_keeps_square_rows() {
    let width = 160;
    let height = 110;
    let mut canvas = vec![0; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![1.0], 0);
    let geometry = BarGeometry::from_visualizer(&VisualizerConfig {
        bar_width: 40,
        bar_corner_radius: 0.0,
        ..VisualizerConfig::default()
    });

    render_bars(
        &mut canvas,
        RenderTarget::new(width, height, 1),
        &frame,
        &OverlayPosition::Bottom,
        &default_paint(),
        &geometry,
    );

    let top_row = first_opaque_row(&canvas, width, height)
        .unwrap_or_else(|| panic!("expected square bar pixels"));
    let top_runs = opaque_runs_in_row(&canvas, width, top_row);

    assert_eq!(top_runs, vec![40]);
}

#[test]
fn render_scale_applies_to_configured_bar_width() {
    let logical_width = 160;
    let logical_height = 110;
    let scale = 2;
    let width = logical_width * scale;
    let height = logical_height * scale;
    let mut canvas = vec![0; (width * height * 4) as usize];
    let frame = SpectrumFrame::new(vec![1.0], 0);
    let geometry = BarGeometry::from_visualizer(&VisualizerConfig {
        bar_width: 40,
        bar_corner_radius: 0.0,
        ..VisualizerConfig::default()
    });

    render_bars(
        &mut canvas,
        RenderTarget::new(width, height, scale),
        &frame,
        &OverlayPosition::Bottom,
        &default_paint(),
        &geometry,
    );

    let top_row = first_opaque_row(&canvas, width, height)
        .unwrap_or_else(|| panic!("expected scaled bar pixels"));
    let top_runs = opaque_runs_in_row(&canvas, width, top_row);

    assert_eq!(top_runs, vec![80]);
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

fn opaque_runs_in_row(canvas: &[u8], width: u32, row: u32) -> Vec<usize> {
    let start = (row * width * 4) as usize;
    let end = start + (width * 4) as usize;
    opaque_runs(
        canvas[start..end]
            .chunks_exact(4)
            .map(|pixel| pixel[3] != 0),
    )
}

fn opaque_runs_in_column(canvas: &[u8], width: u32, column: u32) -> Vec<usize> {
    opaque_runs(
        canvas
            .chunks_exact((width * 4) as usize)
            .map(|row| row[(column * 4) as usize + 3] != 0),
    )
}

fn first_opaque_row(canvas: &[u8], width: u32, height: u32) -> Option<u32> {
    (0..height).find(|row| row_has_opaque_pixels(canvas, width, *row))
}

fn opaque_runs(items: impl Iterator<Item = bool>) -> Vec<usize> {
    let mut runs = Vec::new();
    let mut current = 0;
    for opaque in items {
        if opaque {
            current += 1;
        } else if current > 0 {
            runs.push(current);
            current = 0;
        }
    }
    if current > 0 {
        runs.push(current);
    }
    runs
}

fn default_paint() -> BarPaint {
    BarPaint::from_visualizer(&VisualizerConfig::default(), None)
}

fn default_geometry() -> BarGeometry {
    BarGeometry::from_visualizer(&VisualizerConfig::default())
}

fn square_geometry() -> BarGeometry {
    BarGeometry::from_visualizer(&VisualizerConfig {
        bar_corner_radius: 0.0,
        ..VisualizerConfig::default()
    })
}
