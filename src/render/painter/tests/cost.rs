use std::time::{Duration, Instant};

use crate::config::{SurfaceConfig, parse};
use crate::render::{ByteOrder, Canvas, Painter};

/// the user's outputs: 3840x2160 at scale 1.5 less a margin
const SIZE: (u32, u32) = (3780, 2160);

/// an example preset with some of its lines replaced
fn preset(name: &str, edits: &[(&str, &str)]) -> SurfaceConfig {
    let path = format!("{}/assets/examples/{name}.toml", env!("CARGO_MANIFEST_DIR"));
    let mut raw = std::fs::read_to_string(&path).unwrap_or_default();
    for (from, to) in edits {
        raw = raw.replace(from, to);
    }
    let Ok(parsed) = parse(&raw) else {
        panic!("{path} does not parse");
    };
    parsed.config.surface(None, None)
}

/// `cargo test --release --lib -- --ignored --nocapture paint_cost`, optionally
/// with `KWYBARS_COST_CASE=radial` to run the cases whose name contains it
#[test]
#[ignore = "timing, run in release"]
fn paint_cost() {
    let turning = [
        ("radial_rotation_speed = 0", "radial_rotation_speed = 30"),
        (
            "bar_corner_radius = 16",
            "bar_corner_radius = 3\nsegmented_bars = true\nsegment_length = 14\nsegment_gap = 5",
        ),
        ("bar_width = 10", "bar_width = 14"),
    ];
    let cases = [
        ("line", preset("line", &[])),
        ("particle", preset("particle", &[])),
        ("floating", preset("floating", &[])),
        ("radial", preset("radial", &[])),
        (
            "radial turning",
            preset(
                "radial",
                &[("radial_rotation_speed = 0", "radial_rotation_speed = 30")],
            ),
        ),
        ("radial turning, segmented", preset("radial", &turning)),
        ("wave", preset("wave", &[])),
        (
            "wave with glow",
            preset("wave", &[("wave_glow = false", "wave_glow = true")]),
        ),
        (
            "wave, stroke only",
            preset("wave", &[("wave_fill = true", "wave_fill = false")]),
        ),
        ("polygon", preset("polygon", &[])),
        (
            "polygon turning",
            preset(
                "polygon",
                &[("polygon_rotation_speed = 0", "polygon_rotation_speed = 30")],
            ),
        ),
    ];
    let only = std::env::var("KWYBARS_COST_CASE").unwrap_or_default();
    for (name, surface) in cases
        .into_iter()
        .filter(|(name, _)| name.contains(only.as_str()))
    {
        let bars = surface.visualizer.bars;
        let mut painter = Painter::new(&surface, bars, SIZE, 1.5, ByteOrder::Bgra);
        let mut buffers = [
            vec![0_u8; (SIZE.0 * SIZE.1 * 4) as usize],
            vec![0_u8; (SIZE.0 * SIZE.1 * 4) as usize],
        ];
        let mut contents = [painter.new_contents(), painter.new_contents()];
        let mut heights = vec![0.0_f32; bars];
        let start = Instant::now();
        let frames = 240_u32;
        let mut spent = Duration::ZERO;
        for frame in 0..frames {
            for (bar, height) in heights.iter_mut().enumerate() {
                let phase = frame as f32 * 0.09 + bar as f32 * 0.7;
                *height =
                    0.5 + 0.45 * phase.sin() * (bar as f32 * 0.31 + frame as f32 * 0.02).cos();
            }
            let now = start + Duration::from_micros(16_667 * u64::from(frame));
            let slot = frame as usize % 2;
            let began = Instant::now();
            painter.layout(&heights, 255, now);
            let Some(mut canvas) = Canvas::new(&mut buffers[slot], SIZE) else {
                panic!("canvas");
            };
            painter.paint(&mut canvas, &mut contents[slot]);
            painter.present(|area| {
                std::hint::black_box(area);
            });
            spent += began.elapsed();
        }
        println!(
            "{name}: {:.3} ms per frame",
            spent.as_secs_f64() * 1e3 / f64::from(frames)
        );
    }
}
