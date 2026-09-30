//! the layout a surface draws, chosen from its layout key

use std::ops::Range;

use super::fill::{Axis, Fill};
use super::line::LineLayout;
use super::{Canvas, PixelRect, frame, mirror};
use crate::config::{Config, Layout, SurfaceConfig};

#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    size: (u32, u32),
    parts: Vec<Part>,
    elements: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct Part {
    layout: LineLayout,
    values: Range<usize>,
    axis: Axis,
    first: usize,
}

impl Geometry {
    pub fn new(config: &SurfaceConfig, size: (u32, u32), scale: f32, bars: usize) -> Self {
        let visualizer = &config.visualizer;
        let mut parts = Vec::new();
        let mut first = 0;
        let mut push = |layout: LineLayout, values: Range<usize>, axis: Axis| {
            let count = values.len();
            parts.push(Part {
                layout,
                values,
                axis,
                first,
            });
            first += count;
        };
        match visualizer.layout {
            Layout::Mirror => {
                let (halves, axis) = mirror::strips(config, size, scale);
                for strip in halves {
                    push(
                        LineLayout::new(visualizer, strip, size, scale, bars),
                        0..bars,
                        axis,
                    );
                }
            }
            Layout::Frame => {
                for part in frame::parts(config, size, scale, bars) {
                    let count = part.values.len();
                    let layout = LineLayout::new(visualizer, part.strip, size, scale, count);
                    push(layout, part.values, part.axis);
                }
            }
            Layout::Line
            | Layout::Wave
            | Layout::Radial
            | Layout::Polygon
            | Layout::Particle
            | Layout::Floating => {
                let edge = config.overlay.position;
                let line = LineLayout::along(visualizer, edge, size, scale, bars);
                push(
                    line,
                    0..bars,
                    Axis::along(edge, visualizer.gradient_direction, size),
                );
            }
        }
        Self {
            size,
            parts,
            elements: first,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    /// the gradient line of each part, in part order
    pub fn axes(&self) -> impl Iterator<Item = Axis> + '_ {
        self.parts.iter().map(|part| part.axis)
    }

    /// the bar value `element` shows
    pub fn bar(&self, element: usize) -> usize {
        self.find(element)
            .map_or(0, |(_, part, local)| part.values.start + local)
    }

    pub fn animates(&self) -> bool {
        false
    }

    /// how far `element` reaches at `value` in `0.0..=1.0`, in pixels
    pub fn extent(&self, element: usize, value: f32) -> f32 {
        self.find(element)
            .map_or(0.0, |(_, part, _)| part.layout.extent(value))
    }

    /// every pixel of `element` at `extent`
    pub fn area(&self, element: usize, extent: f32) -> Option<PixelRect> {
        let (_, part, local) = self.find(element)?;
        part.layout.area(local, extent)
    }

    /// the pixels that differ between `element` at `old` and at `new`
    pub fn change(&self, element: usize, old: f32, new: f32) -> Option<PixelRect> {
        let (_, part, local) = self.find(element)?;
        part.layout.change(local, old, new)
    }

    pub fn paint(
        &self,
        canvas: &mut Canvas<'_>,
        element: usize,
        extent: f32,
        clip: PixelRect,
        fills: &[Fill],
    ) {
        let Some((index, part, local)) = self.find(element) else {
            return;
        };
        if let Some(fill) = fills.get(index) {
            let color = part.values.start + local;
            part.layout.paint(canvas, local, color, extent, clip, fill);
        }
    }

    /// the part holding `element`, its index, and the element's bar in it
    fn find(&self, element: usize) -> Option<(usize, &Part, usize)> {
        self.parts.iter().enumerate().find_map(|(index, part)| {
            let local = element.checked_sub(part.first)?;
            (local < part.values.len()).then_some((index, part, local))
        })
    }
}

/// whether layout has its own drawing yet
pub fn is_ported(layout: Layout) -> bool {
    matches!(layout, Layout::Line | Layout::Mirror | Layout::Frame)
}

/// one warning per table that asks for a layout not drawn yet
pub fn unported_warnings(config: &Config) -> Vec<String> {
    let global = (!is_ported(config.visualizer.layout)).then(|| {
        format!(
            "visualizer.layout: {:?} is not drawn yet in this version, drawing line",
            config.visualizer.layout
        )
    });
    let outputs = config.overlay.outputs.iter().filter_map(|output| {
        let layout = output.visualizer.layout?;
        (!is_ported(layout)).then(|| {
            format!(
                "overlay.outputs {:?}: layout {layout:?} is not drawn yet in this version, drawing line",
                output.monitor
            )
        })
    });
    global.into_iter().chain(outputs).collect()
}

#[cfg(test)]
mod tests {
    use super::unported_warnings;
    use crate::config::{Config, Layout, OutputConfig};

    #[test]
    fn warns_once_per_table_with_an_unported_layout() {
        let mut config = Config::default();
        assert!(unported_warnings(&config).is_empty());
        config.visualizer.layout = Layout::Radial;
        let mut output = OutputConfig {
            monitor: "DP-2".to_owned(),
            ..OutputConfig::default()
        };
        output.visualizer.layout = Some(Layout::Wave);
        config.overlay.outputs = vec![output, OutputConfig::default()];
        let warnings = unported_warnings(&config);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].contains("Radial") && warnings[1].contains("DP-2"));
    }
}
