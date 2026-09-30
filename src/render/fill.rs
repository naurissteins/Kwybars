//! where each bar pixel's color comes from, following the legacy layouts

use super::ByteOrder;
use crate::config::{ColorMode, Edge, GradientDirection, Rgba, SurfaceConfig};

/// the line a gradient runs along, in buffer pixels; pixels before `start`
/// or past its end take the end colors
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis {
    /// along x, one color per column; else along y, one per row
    pub columns: bool,
    pub start: f32,
    pub length: f32,
    /// the first stop is at the end
    pub reversed: bool,
}

impl Axis {
    /// `line`: across the whole buffer, a vertical gradient starting at the
    /// bars' base on `edge`
    pub fn along(edge: Edge, direction: GradientDirection, (width, height): (u32, u32)) -> Self {
        let along_bars = matches!(edge, Edge::Bottom | Edge::Top);
        let columns = along_bars == (direction == GradientDirection::Horizontal);
        Self {
            columns,
            start: 0.0,
            length: if columns { width } else { height } as f32,
            reversed: direction == GradientDirection::Vertical
                && matches!(edge, Edge::Bottom | Edge::Right),
        }
    }
}

/// premultiplied colors in buffer byte order, prepared once per size
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    Solid([u8; 4]),
    Columns(Vec<[u8; 4]>),
    Rows(Vec<[u8; 4]>),
    Bars(Vec<[u8; 4]>),
}

impl Fill {
    /// the color source for a buffer of `size`, gradients running along `axis`
    pub fn new(
        config: &SurfaceConfig,
        axis: Axis,
        (width, height): (u32, u32),
        bars: usize,
        order: ByteOrder,
    ) -> Self {
        let visualizer = &config.visualizer;
        let direction = visualizer.gradient_direction;
        let stops: &[Rgba] = match &config.theme_colors {
            Some(colors) if direction == GradientDirection::Vertical => {
                let colors = (0..bars)
                    .map(|bar| {
                        let index = bar_color_index(bar, bars, colors.len());
                        order.pack(colors.get(index).copied().unwrap_or(visualizer.color_rgba))
                    })
                    .collect();
                return Self::Bars(colors);
            }
            Some(colors) => colors,
            None if visualizer.color_mode == ColorMode::Solid => {
                return Self::Solid(order.pack(visualizer.color_rgba));
            }
            None => &[visualizer.color_rgba, visualizer.color2_rgba],
        };
        if let [only] = stops {
            return Self::Solid(order.pack(*only));
        }

        let pixels = if axis.columns { width } else { height };
        let table = (0..pixels)
            .map(|pixel| {
                let along = (pixel as f32 + 0.5 - axis.start) / axis.length.max(1.0);
                let t = if axis.reversed { 1.0 - along } else { along };
                order.pack(gradient(stops, t))
            })
            .collect();
        if axis.columns {
            Self::Columns(table)
        } else {
            Self::Rows(table)
        }
    }

    /// the color of bar `bar` at pixel `x`, `y`
    #[inline]
    pub fn color(&self, bar: usize, x: u32, y: u32) -> [u8; 4] {
        let pick = |table: &[[u8; 4]], index: usize| table.get(index).copied().unwrap_or([0; 4]);
        match self {
            Self::Solid(color) => *color,
            Self::Columns(table) => pick(table, x as usize),
            Self::Rows(table) => pick(table, y as usize),
            Self::Bars(table) => pick(table, bar),
        }
    }
}

impl Fill {
    /// writes the colors of bar `bar` for row `y` from column `x0` into `out`
    #[inline]
    pub fn span(&self, bar: usize, x0: u32, y: u32, out: &mut [[u8; 4]]) {
        match self {
            Self::Columns(table) => {
                let start = x0 as usize;
                match table.get(start..start + out.len()) {
                    Some(colors) => out.copy_from_slice(colors),
                    None => out.fill([0; 4]),
                }
            }
            _ => out.fill(self.color(bar, x0, y)),
        }
    }
}

impl Fill {
    /// becomes `base` at `opacity` out of 255; `base` must be a clone of this
    /// fill, so the tables match and nothing is allocated
    pub fn fade_from(&mut self, base: &Self, opacity: u8) {
        let scale = |colors: &mut [[u8; 4]], base: &[[u8; 4]]| {
            for (color, base) in colors.iter_mut().zip(base) {
                for (channel, value) in color.iter_mut().zip(base) {
                    // premultiplied, so every channel scales alike
                    *channel = ((u16::from(*value) * u16::from(opacity) + 127) / 255) as u8;
                }
            }
        };
        match (self, base) {
            (Self::Solid(color), Self::Solid(base)) => {
                scale(std::slice::from_mut(color), std::slice::from_ref(base));
            }
            (Self::Columns(colors), Self::Columns(base))
            | (Self::Rows(colors), Self::Rows(base))
            | (Self::Bars(colors), Self::Bars(base)) => scale(colors, base),
            _ => {}
        }
    }
}

/// which of `color_count` palette colors bar `bar_index` of `bar_count` gets,
/// in even blocks
pub fn bar_color_index(bar_index: usize, bar_count: usize, color_count: usize) -> usize {
    if bar_count == 0 || color_count == 0 {
        return 0;
    }
    (bar_index.saturating_mul(color_count) / bar_count).min(color_count - 1)
}

/// evenly spaced `stops` sampled at `t` in `0.0..=1.0`
fn gradient(stops: &[Rgba], t: f32) -> Rgba {
    let Some(last) = stops.len().checked_sub(1) else {
        return Rgba::new(0.0, 0.0, 0.0, 0.0);
    };
    let scaled = t.clamp(0.0, 1.0) * last as f32;
    let lower = (scaled.floor() as usize).min(last);
    let upper = (lower + 1).min(last);
    let (Some(a), Some(b)) = (stops.get(lower), stops.get(upper)) else {
        return Rgba::new(0.0, 0.0, 0.0, 0.0);
    };
    let f = scaled - lower as f32;
    let lerp = |x: f32, y: f32| x + (y - x) * f;
    Rgba::new(
        lerp(a.r, b.r),
        lerp(a.g, b.g),
        lerp(a.b, b.b),
        lerp(a.a, b.a),
    )
}

#[cfg(test)]
mod tests {
    use super::{Axis, Fill, bar_color_index, gradient};
    use crate::config::{ColorMode, Config, Edge, GradientDirection, Rgba, SurfaceConfig};
    use crate::render::ByteOrder;

    const RED: Rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);
    const BLUE: Rgba = Rgba::new(0.0, 0.0, 1.0, 1.0);

    fn surface(mode: ColorMode, direction: GradientDirection, theme: bool) -> SurfaceConfig {
        let mut config = Config::default();
        config.visualizer.color_mode = mode;
        config.visualizer.gradient_direction = direction;
        config.visualizer.color_rgba = RED;
        config.visualizer.color2_rgba = BLUE;
        let mut surface = config.surface(None, None);
        if theme {
            surface.theme_colors = Some([RED, RED, RED, BLUE, BLUE, BLUE]);
        }
        surface
    }

    #[test]
    fn spreads_colors_evenly() {
        let indices: Vec<usize> = (0..12).map(|index| bar_color_index(index, 12, 6)).collect();
        assert_eq!(indices, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
    }

    #[test]
    fn gradient_interpolates_between_even_stops() {
        let mid = gradient(&[RED, BLUE], 0.5);
        assert_eq!((mid.r, mid.b), (0.5, 0.5));
        assert_eq!(gradient(&[RED, BLUE, RED], 0.5), BLUE);
        assert_eq!(gradient(&[RED, BLUE], 7.0), BLUE);
    }

    #[test]
    fn solid_mode_is_one_color() {
        let fill = Fill::new(
            &surface(ColorMode::Solid, GradientDirection::Vertical, false),
            Axis::along(
                Edge::Bottom,
                surface(ColorMode::Solid, GradientDirection::Vertical, false)
                    .visualizer
                    .gradient_direction,
                (10, 10),
            ),
            (10, 10),
            4,
            ByteOrder::Rgba,
        );
        assert_eq!(fill, Fill::Solid([255, 0, 0, 255]));
    }

    #[test]
    fn vertical_gradient_starts_at_the_bars_base() {
        let config = surface(ColorMode::Gradient, GradientDirection::Vertical, false);
        let bottom = Fill::new(
            &config,
            Axis::along(Edge::Bottom, config.visualizer.gradient_direction, (10, 4)),
            (10, 4),
            4,
            ByteOrder::Rgba,
        );
        // rows from the top: mostly blue at the tip side, mostly red at the base
        assert!(matches!(&bottom, Fill::Rows(rows) if rows[0][2] > 200 && rows[3][0] > 200));
        let top = Fill::new(
            &config,
            Axis::along(Edge::Top, config.visualizer.gradient_direction, (10, 4)),
            (10, 4),
            4,
            ByteOrder::Rgba,
        );
        assert!(matches!(&top, Fill::Rows(rows) if rows[0][0] > 200 && rows[3][2] > 200));
        let right = Fill::new(
            &config,
            Axis::along(Edge::Right, config.visualizer.gradient_direction, (4, 10)),
            (4, 10),
            4,
            ByteOrder::Rgba,
        );
        assert!(matches!(&right, Fill::Columns(cols) if cols[3][0] > 200));
    }

    #[test]
    fn horizontal_gradient_runs_along_the_bars() {
        let config = surface(ColorMode::Gradient, GradientDirection::Horizontal, true);
        let bottom = Fill::new(
            &config,
            Axis::along(Edge::Bottom, config.visualizer.gradient_direction, (6, 3)),
            (6, 3),
            4,
            ByteOrder::Rgba,
        );
        assert!(
            matches!(&bottom, Fill::Columns(cols) if cols.len() == 6 && cols[0][0] == 255 && cols[5][2] == 255)
        );
        let left = Fill::new(
            &config,
            Axis::along(Edge::Left, config.visualizer.gradient_direction, (3, 6)),
            (3, 6),
            4,
            ByteOrder::Rgba,
        );
        assert!(matches!(&left, Fill::Rows(rows) if rows.len() == 6));
    }

    #[test]
    fn fading_scales_every_premultiplied_channel() {
        let config = surface(ColorMode::Gradient, GradientDirection::Horizontal, false);
        let base = Fill::new(
            &config,
            Axis::along(Edge::Bottom, config.visualizer.gradient_direction, (4, 4)),
            (4, 4),
            4,
            ByteOrder::Rgba,
        );
        let mut faded = base.clone();
        faded.fade_from(&base, 128);
        let (Fill::Columns(base_colors), Fill::Columns(colors)) = (&base, &faded) else {
            panic!("expected columns");
        };
        assert_eq!(base_colors[0], [223, 0, 32, 255]);
        assert_eq!(colors[0], [112, 0, 16, 128]);
        faded.fade_from(&base, 255);
        assert_eq!(faded, base);
        faded.fade_from(&base, 0);
        assert_eq!(faded.color(0, 0, 0), [0; 4]);
    }

    #[test]
    fn theme_with_vertical_gradient_colors_whole_bars() {
        let config = surface(ColorMode::Gradient, GradientDirection::Vertical, true);
        let fill = Fill::new(
            &config,
            Axis::along(Edge::Bottom, config.visualizer.gradient_direction, (10, 10)),
            (10, 10),
            4,
            ByteOrder::Bgra,
        );
        let blue_bgra = [255, 0, 0, 255];
        let red_bgra = [0, 0, 255, 255];
        assert_eq!(
            fill,
            Fill::Bars(vec![red_bgra, red_bgra, blue_bgra, blue_bgra])
        );
        assert_eq!(fill.color(3, 0, 0), blue_bgra);
    }
}
