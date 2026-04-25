use kwybars_common::config::{RgbaColor, VisualizerColorMode, VisualizerGradientDirection};

#[derive(Clone, Copy)]
pub(super) struct GradientAxis {
    pub(super) start: (f64, f64),
    pub(super) end: (f64, f64),
}

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

pub(super) fn gradient_axis_for_layout(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    sequence_horizontal: bool,
    growth_from_start: bool,
    direction: VisualizerGradientDirection,
) -> GradientAxis {
    match direction {
        VisualizerGradientDirection::Vertical => {
            if sequence_horizontal {
                if growth_from_start {
                    GradientAxis {
                        start: (x, y),
                        end: (x, y + height),
                    }
                } else {
                    GradientAxis {
                        start: (x, y + height),
                        end: (x, y),
                    }
                }
            } else if growth_from_start {
                GradientAxis {
                    start: (x, y),
                    end: (x + width, y),
                }
            } else {
                GradientAxis {
                    start: (x + width, y),
                    end: (x, y),
                }
            }
        }
        VisualizerGradientDirection::Horizontal => {
            if sequence_horizontal {
                GradientAxis {
                    start: (x, y),
                    end: (x + width, y),
                }
            } else {
                GradientAxis {
                    start: (x, y),
                    end: (x, y + height),
                }
            }
        }
    }
}

pub(super) fn set_gradient_source(
    ctx: &gtk::cairo::Context,
    axis: GradientAxis,
    color_mode: VisualizerColorMode,
    color: RgbaColor,
    color2: RgbaColor,
    theme_colors: Option<&[RgbaColor]>,
    alpha_scale: f64,
) {
    if let Some(colors) = theme_colors {
        if colors.len() == 1 {
            let (r, g, b, a) = scaled_rgba(colors[0], alpha_scale);
            ctx.set_source_rgba(r, g, b, a);
            return;
        }

        let gradient =
            gtk::cairo::LinearGradient::new(axis.start.0, axis.start.1, axis.end.0, axis.end.1);
        let stop_denom = (colors.len().saturating_sub(1)).max(1) as f64;
        for (index, stop_color) in colors.iter().enumerate() {
            let (r, g, b, a) = scaled_rgba(*stop_color, alpha_scale);
            gradient.add_color_stop_rgba(index as f64 / stop_denom, r, g, b, a);
        }

        if ctx.set_source(&gradient).is_ok() {
            return;
        }

        let (r, g, b, a) = scaled_rgba(colors[0], alpha_scale);
        ctx.set_source_rgba(r, g, b, a);
        return;
    }

    match color_mode {
        VisualizerColorMode::Solid => {
            let (r, g, b, a) = scaled_rgba(color, alpha_scale);
            ctx.set_source_rgba(r, g, b, a);
        }
        VisualizerColorMode::Gradient => {
            let gradient =
                gtk::cairo::LinearGradient::new(axis.start.0, axis.start.1, axis.end.0, axis.end.1);
            let (r1, g1, b1, a1) = scaled_rgba(color, alpha_scale);
            let (r2, g2, b2, a2) = scaled_rgba(color2, alpha_scale);
            gradient.add_color_stop_rgba(0.0, r1, g1, b1, a1);
            gradient.add_color_stop_rgba(1.0, r2, g2, b2, a2);
            if ctx.set_source(&gradient).is_err() {
                ctx.set_source_rgba(r1, g1, b1, a1);
            }
        }
    }
}

pub(super) fn scaled_rgba(color: RgbaColor, alpha_scale: f64) -> (f64, f64, f64, f64) {
    (
        f64::from(color.r),
        f64::from(color.g),
        f64::from(color.b),
        (f64::from(color.a) * alpha_scale).clamp(0.0, 1.0),
    )
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + ((end - start) * t.clamp(0.0, 1.0))
}
