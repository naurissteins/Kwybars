#[cfg(test)]
mod tests;

use super::ByteOrder;
use crate::config::{ColorMode, Edge, GradientDirection, Rgba, SurfaceConfig};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis {
    pub columns: bool,
    pub start: f32,
    pub length: f32,
    pub reversed: bool,
}

impl Axis {
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
    /// the color source for a buffer of size, gradients running along axis
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

    pub fn along(
        config: &SurfaceConfig,
        columns: bool,
        length: u32,
        order: ByteOrder,
        alpha: f32,
    ) -> Self {
        let visualizer = &config.visualizer;
        let pair = [visualizer.color_rgba, visualizer.color2_rgba];
        let stops: &[Rgba] = match &config.theme_colors {
            Some(colors) => colors,
            None if visualizer.color_mode == ColorMode::Solid => &pair[..1],
            None => &pair,
        };
        let scaled =
            |color: Rgba| Rgba::new(color.r, color.g, color.b, (color.a * alpha).clamp(0.0, 1.0));
        if let [only] = stops {
            return Self::Solid(order.pack(scaled(*only)));
        }
        let table = (0..length)
            .map(|pixel| {
                let t = (pixel as f32 + 0.5) / length.max(1) as f32;
                order.pack(scaled(gradient(stops, t)))
            })
            .collect();
        if columns {
            Self::Columns(table)
        } else {
            Self::Rows(table)
        }
    }

    /// smooth_theme blends the palette from bar to bar instead of in blocks
    pub fn per_bar(
        config: &SurfaceConfig,
        bars: usize,
        order: ByteOrder,
        smooth_theme: bool,
    ) -> Self {
        let visualizer = &config.visualizer;
        let (first, last) = (visualizer.color_rgba, visualizer.color2_rgba);
        let along = |bar: usize| bar as f32 / bars.saturating_sub(1).max(1) as f32;
        let colors = match &config.theme_colors {
            Some(colors) if smooth_theme => (0..bars)
                .map(|bar| order.pack(gradient(colors, along(bar))))
                .collect(),
            Some(colors) => (0..bars)
                .map(|bar| {
                    let index = bar_color_index(bar, bars, colors.len());
                    order.pack(colors.get(index).copied().unwrap_or(first))
                })
                .collect(),
            None if visualizer.color_mode == ColorMode::Solid || bars <= 1 => {
                return Self::Solid(order.pack(first));
            }
            None => (0..bars)
                .map(|bar| order.pack(gradient(&[first, last], along(bar))))
                .collect(),
        };
        Self::Bars(colors)
    }

    /// the color of bar bar at pixel x, y
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
    /// the colors of len columns from x0, when each column has its own
    pub fn columns(&self, x0: u32, len: usize) -> Option<&[[u8; 4]]> {
        match self {
            Self::Columns(table) => table.get(x0 as usize..x0 as usize + len),
            _ => None,
        }
    }

    /// the color of bar along all of row y, unless it changes along the row
    #[inline]
    pub fn row(&self, bar: usize, y: u32) -> Option<[u8; 4]> {
        match self {
            Self::Columns(_) => None,
            _ => Some(self.color(bar, 0, y)),
        }
    }

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

pub fn bar_color_index(bar_index: usize, bar_count: usize, color_count: usize) -> usize {
    if bar_count == 0 || color_count == 0 {
        return 0;
    }
    (bar_index.saturating_mul(color_count) / bar_count).min(color_count - 1)
}

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
