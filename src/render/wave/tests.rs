use std::time::{Duration, Instant};

use super::Wave;
use crate::config::{ColorMode, Config, Edge, Rgba, SurfaceConfig};
use crate::render::{ByteOrder, Canvas};

const SIZE: (u32, u32) = (120, 60);

fn surface(edge: Edge, edit: impl FnOnce(&mut Config)) -> SurfaceConfig {
    let mut config = Config::default();
    config.overlay.position = edge;
    config.visualizer.wave_stroke_width = 4;
    config.visualizer.wave_fill = false;
    config.visualizer.wave_glow = false;
    config.visualizer.wave_motion_smoothing = 1.0;
    config.visualizer.color_mode = ColorMode::Solid;
    config.visualizer.color_rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);
    edit(&mut config);
    config.surface(None, None)
}

/// a wave of five values that has arrived at them, drawn on a clear buffer
fn drawn(surface: &SurfaceConfig, size: (u32, u32), heights: &[f32; 5]) -> (Wave, Vec<u8>) {
    let mut wave = Wave::new(surface, size, 1.0, 5);
    let start = Instant::now();
    for frame in 0..4 {
        wave.step(heights, start + Duration::from_millis(16 * frame));
    }
    let mut data = vec![0_u8; (size.0 * size.1 * 4) as usize];
    let Some(mut canvas) = Canvas::new(&mut data, size) else {
        panic!("canvas");
    };
    let fills = Wave::fills(surface, size, ByteOrder::Rgba);
    wave.paint(&mut canvas, &fills, &mut wave.held(), true);
    (wave, data)
}

fn pixel(data: &[u8], size: (u32, u32), x: u32, y: u32) -> [u8; 4] {
    let at = ((y * size.0 + x) * 4) as usize;
    [data[at], data[at + 1], data[at + 2], data[at + 3]]
}

#[test]
fn a_flat_wave_is_a_line_through_the_middle() {
    let (wave, data) = drawn(&surface(Edge::Bottom, |_| {}), SIZE, &[0.5; 5]);
    // padding 2 + 6: from x 8 to 112 at y 30, 4 wide
    assert_eq!(pixel(&data, SIZE, 60, 29), [255, 0, 0, 255]);
    assert_eq!(pixel(&data, SIZE, 60, 30), [255, 0, 0, 255]);
    assert_eq!(pixel(&data, SIZE, 60, 27), [0; 4]);
    assert_eq!(pixel(&data, SIZE, 60, 33), [0; 4]);
    assert_eq!(pixel(&data, SIZE, 3, 30), [0; 4]);
    let Some(bounds) = wave.bounds() else {
        panic!("a drawn wave has bounds");
    };
    assert!(bounds.y >= 25 && bounds.bottom() <= 35, "{bounds:?}");
    // nothing drawn outside the bounds
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            let inside =
                (bounds.x..bounds.right()).contains(&x) && (bounds.y..bounds.bottom()).contains(&y);
            assert!(inside || pixel(&data, SIZE, x, y) == [0; 4], "{x}, {y}");
        }
    }
}

#[test]
fn the_fill_reaches_from_the_curve_to_the_baseline() {
    let filled = |edge| {
        surface(edge, |c| {
            c.visualizer.wave_fill = true;
            c.visualizer.color_rgba = Rgba::new(1.0, 0.0, 0.0, 0.5);
        })
    };
    // the fill is the color at 24% of its alpha: 0.12
    let fill = [31, 0, 0, 31];
    let (_, bottom) = drawn(&filled(Edge::Bottom), SIZE, &[0.5; 5]);
    assert_eq!(pixel(&bottom, SIZE, 60, 40), fill);
    assert_eq!(pixel(&bottom, SIZE, 60, 51), fill);
    // the baseline is the padding from the edge
    assert_eq!(pixel(&bottom, SIZE, 60, 52), [0; 4]);
    assert_eq!(pixel(&bottom, SIZE, 60, 20), [0; 4]);
    // the stroke goes over the fill: 0.5 over 0.12
    assert_eq!(pixel(&bottom, SIZE, 60, 30), [143, 0, 0, 143]);
    let (_, top) = drawn(&filled(Edge::Top), SIZE, &[0.5; 5]);
    assert_eq!(pixel(&top, SIZE, 60, 20), fill);
    assert_eq!(pixel(&top, SIZE, 60, 8), fill);
    assert_eq!(pixel(&top, SIZE, 60, 7), [0; 4]);
    assert_eq!(pixel(&top, SIZE, 60, 40), [0; 4]);
    let size = (60, 120);
    let (_, left) = drawn(&filled(Edge::Left), size, &[0.5; 5]);
    assert_eq!(pixel(&left, size, 20, 60), fill);
    assert_eq!(pixel(&left, size, 40, 60), [0; 4]);
    let (_, right) = drawn(&filled(Edge::Right), size, &[0.5; 5]);
    assert_eq!(pixel(&right, size, 40, 60), fill);
    assert_eq!(pixel(&right, size, 20, 60), [0; 4]);
}

#[test]
fn the_glow_is_a_wider_fainter_stroke_under_the_stroke() {
    let glowing = surface(Edge::Bottom, |c| c.visualizer.wave_glow = true);
    let (_, data) = drawn(&glowing, SIZE, &[0.5; 5]);
    // glow 12 wide at 18% alpha, the stroke 4 wide over it
    assert_eq!(pixel(&data, SIZE, 60, 30), [255, 0, 0, 255]);
    assert_eq!(pixel(&data, SIZE, 60, 26), [46, 0, 0, 46]);
    assert_eq!(pixel(&data, SIZE, 60, 34), [46, 0, 0, 46]);
    assert_eq!(pixel(&data, SIZE, 60, 22), [0; 4]);
}

#[test]
fn louder_bars_swing_away_from_the_edge() {
    let (_, data) = drawn(
        &surface(Edge::Bottom, |_| {}),
        SIZE,
        &[0.0, 0.0, 1.0, 0.0, 0.0],
    );
    // the middle point rises above the middle, its neighbors sink below it
    let first_lit = |x: u32| (0..SIZE.1).find(|y| pixel(&data, SIZE, x, *y)[3] == 255);
    let (Some(peak), Some(side)) = (first_lit(60), first_lit(20)) else {
        panic!("the stroke crosses both columns");
    };
    assert!(peak < 20 && side > 30, "peak {peak}, side {side}");
}

#[test]
fn values_follow_their_bars_at_any_framerate() {
    let surface = surface(Edge::Bottom, |c| c.visualizer.wave_motion_smoothing = 0.2);
    let after = |fps: u64| {
        let mut wave = Wave::new(&surface, SIZE, 1.0, 5);
        let start = Instant::now();
        for frame in 0..=fps / 4 {
            let now = start + Duration::from_micros(frame * 1_000_000 / fps);
            wave.step(&[1.0, 0.0, 0.5, 0.0, 0.0], now);
        }
        wave.values[0]
    };
    // legacy at 60 fps: 15 ticks of 0.2 x 1.35 towards 1
    let legacy = 1.0 - (1.0_f32 - 0.27).powi(15);
    assert!(
        (after(60) - legacy).abs() < 1e-3,
        "{} vs {legacy}",
        after(60)
    );
    assert!((after(144) - legacy).abs() < 2e-3 && (after(30) - legacy).abs() < 0.02);
}

#[test]
fn the_wave_moves_until_its_values_arrive() {
    let surface = surface(Edge::Bottom, |c| c.visualizer.wave_motion_smoothing = 0.3);
    let mut wave = Wave::new(&surface, SIZE, 1.0, 5);
    let start = Instant::now();
    let at = |frame: u64| start + Duration::from_millis(16 * frame);
    wave.step(&[0.0; 5], at(0));
    assert!(!wave.moving());
    let quiet = wave.version();
    wave.step(&[0.8, 0.1, 0.4, 0.0, 0.9], at(1));
    assert!(wave.moving() && wave.version() > quiet);
    // falling back takes longer than rising, and ends
    let mut frame = 2;
    while wave.moving() && frame < 2_000 {
        wave.step(&[0.0; 5], at(frame));
        frame += 1;
    }
    assert!(!wave.moving() && frame > 10, "{frame} frames");
    assert_eq!(wave.values, vec![0.0; 5]);
    let settled = wave.version();
    wave.step(&[0.0; 5], at(frame));
    assert_eq!(wave.version(), settled);
    // without motion smoothing the values never leave zero, as in legacy
    let frozen = surface_with_smoothing(0.0);
    let mut wave = Wave::new(&frozen, SIZE, 1.0, 5);
    wave.step(&[1.0; 5], at(0));
    wave.step(&[1.0; 5], at(1));
    assert!(!wave.moving() && wave.values == vec![0.0; 5]);
}

fn surface_with_smoothing(smoothing: f32) -> SurfaceConfig {
    surface(Edge::Bottom, |c| {
        c.visualizer.wave_motion_smoothing = smoothing
    })
}

#[test]
#[ignore = "needs KWYBARS_RASTER_DIR"]
fn dumps_waves_for_comparison() {
    let Some(dir) = std::env::var_os("KWYBARS_RASTER_DIR") else {
        panic!("set KWYBARS_RASTER_DIR to an empty directory");
    };
    let dir = std::path::PathBuf::from(dir);
    let heights = [0.2, 0.9, 0.35, 0.6, 0.05];
    // name, edge, buffer size, stroke width, fill, glow
    let cases = [
        ("bottom-fill-glow", Edge::Bottom, (900, 330), 6, true, true),
        ("top-fill", Edge::Top, (900, 330), 6, true, false),
        ("left-glow", Edge::Left, (330, 900), 10, false, true),
        ("right-thin", Edge::Right, (330, 900), 1, true, false),
    ];
    let mut manifest = String::new();
    for (name, edge, size, stroke, fill, glow) in cases {
        let surface = surface(edge, |c| {
            c.visualizer.wave_stroke_width = stroke;
            c.visualizer.wave_fill = fill;
            c.visualizer.wave_glow = glow;
            c.visualizer.color_mode = ColorMode::Gradient;
            c.visualizer.color_rgba = Rgba::new(0.49, 0.81, 1.0, 0.78);
            c.visualizer.color2_rgba = Rgba::new(0.75, 0.52, 0.99, 0.78);
        });
        let mut wave = Wave::new(&surface, size, 1.5, 5);
        let start = Instant::now();
        for frame in 0..4 {
            wave.step(&heights, start + Duration::from_millis(16 * frame));
        }
        let mut data = vec![0_u8; (size.0 * size.1 * 4) as usize];
        let Some(mut canvas) = Canvas::new(&mut data, size) else {
            panic!("canvas");
        };
        let fills = Wave::fills(&surface, size, ByteOrder::Rgba);
        wave.paint(&mut canvas, &fills, &mut wave.held(), true);
        let written = std::fs::write(dir.join(format!("{name}.raw")), &data);
        assert!(written.is_ok(), "{written:?}");
        let shape = &wave.shape;
        let points: Vec<String> = wave
            .points
            .iter()
            .map(|point| format!("{},{}", point.0, point.1))
            .collect();
        manifest.push_str(&format!(
            "{name} {} {} {} {} {} {} {} {} {}\n",
            size.0,
            size.1,
            u8::from(shape.along_x),
            shape.stroke,
            shape.glow.unwrap_or(0.0),
            shape.baseline.map_or(-1.0, f32::round),
            shape.control,
            u8::from(fill),
            points.join(";")
        ));
    }
    let written = std::fs::write(dir.join("waves.txt"), manifest);
    assert!(written.is_ok(), "{written:?}");
}
