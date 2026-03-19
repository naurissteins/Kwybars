use kwybars_common::config::{RgbaColor, VisualizerColorMode};

pub(super) fn color_for_index(
    mode: VisualizerColorMode,
    start: RgbaColor,
    end: RgbaColor,
    index: usize,
    count: usize,
) -> RgbaColor {
    if mode == VisualizerColorMode::Solid || count <= 1 {
        return start;
    }

    let t = index as f32 / (count.saturating_sub(1)) as f32;
    RgbaColor {
        r: lerp(start.r, end.r, t),
        g: lerp(start.g, end.g, t),
        b: lerp(start.b, end.b, t),
        a: lerp(start.a, end.a, t),
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + ((end - start) * t.clamp(0.0, 1.0))
}
