use kwybars_common::config::{RgbaColor, VisualizerColorMode, VisualizerConfig};

#[derive(Debug, Clone, PartialEq)]
pub struct BarPaint {
    color_mode: VisualizerColorMode,
    color: RgbaColor,
    color2: RgbaColor,
    theme_colors: Vec<RgbaColor>,
}

impl BarPaint {
    pub fn from_visualizer(
        visualizer: &VisualizerConfig,
        theme_colors: Option<Vec<RgbaColor>>,
    ) -> Self {
        Self {
            color_mode: visualizer.color_mode,
            color: visualizer.color_rgba,
            color2: visualizer.color2_rgba,
            theme_colors: theme_colors.unwrap_or_default(),
        }
    }

    pub fn color_for_bar(&self, index: usize, count: usize) -> u32 {
        if !self.theme_colors.is_empty() {
            let color_index = bar_color_index(index, count, self.theme_colors.len());
            return rgba_to_argb(self.theme_colors[color_index]);
        }

        rgba_to_argb(color_for_index(
            self.color_mode,
            self.color,
            self.color2,
            index,
            count,
        ))
    }
}

fn color_for_index(
    mode: VisualizerColorMode,
    start: RgbaColor,
    end: RgbaColor,
    index: usize,
    count: usize,
) -> RgbaColor {
    if mode == VisualizerColorMode::Solid || count <= 1 {
        return start;
    }

    let t = index as f32 / count.saturating_sub(1) as f32;
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

fn bar_color_index(bar_index: usize, bar_count: usize, color_count: usize) -> usize {
    if bar_count == 0 || color_count == 0 {
        return 0;
    }
    let index = bar_index.saturating_mul(color_count) / bar_count;
    index.min(color_count - 1)
}

fn rgba_to_argb(color: RgbaColor) -> u32 {
    let alpha = color.a.clamp(0.0, 1.0);
    let red = premultiply(color.r, alpha);
    let green = premultiply(color.g, alpha);
    let blue = premultiply(color.b, alpha);
    let alpha = to_byte(alpha);

    u32::from(alpha) << 24 | u32::from(red) << 16 | u32::from(green) << 8 | u32::from(blue)
}

fn premultiply(value: f32, alpha: f32) -> u8 {
    to_byte(value.clamp(0.0, 1.0) * alpha)
}

fn to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
