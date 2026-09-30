//! the layout a surface draws, chosen from its layout key

mod shape;

use std::ops::Range;

use super::dots::DotLayout;
use super::fill::{Axis, Fill};
use super::line::LineLayout;
use super::{ByteOrder, Canvas, PixelRect, Pose, frame, mirror};
use crate::config::{Config, Layout, SurfaceConfig};
use shape::Shape;

#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    size: (u32, u32),
    parts: Vec<Part>,
    elements: usize,
    drifting: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Part {
    shape: Shape,
    values: Range<usize>,
    first: usize,
}

impl Geometry {
    pub fn new(config: &SurfaceConfig, size: (u32, u32), scale: f32, bars: usize) -> Self {
        let visualizer = &config.visualizer;
        let edge = config.overlay.position;
        let mut parts = Vec::new();
        let mut first = 0;
        let mut push = |shape: Shape, values: Range<usize>| {
            let count = values.len();
            parts.push(Part {
                shape,
                values,
                first,
            });
            first += count;
        };
        match visualizer.layout {
            Layout::Mirror => {
                let (halves, axis) = mirror::strips(config, size, scale);
                for strip in halves {
                    let layout = LineLayout::new(visualizer, strip, size, scale, bars);
                    push(Shape::Strip(layout, axis), 0..bars);
                }
            }
            Layout::Frame => {
                for part in frame::parts(config, size, scale, bars) {
                    let count = part.values.len();
                    let layout = LineLayout::new(visualizer, part.strip, size, scale, count);
                    push(Shape::Strip(layout, part.axis), part.values);
                }
            }
            Layout::Particle | Layout::Floating => {
                let drifting = visualizer.layout == Layout::Floating;
                let dots = DotLayout::new(visualizer, edge, size, scale, bars, drifting);
                push(Shape::Dots(dots), 0..bars);
            }
            Layout::Line | Layout::Wave | Layout::Radial | Layout::Polygon => {
                let line = LineLayout::along(visualizer, edge, size, scale, bars);
                let axis = Axis::along(edge, visualizer.gradient_direction, size);
                push(Shape::Strip(line, axis), 0..bars);
            }
        }
        Self {
            size,
            parts,
            elements: first,
            drifting: visualizer.layout == Layout::Floating,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    /// whether elements drift on their own and need a [`super::dots::Drift`]
    pub fn drifting(&self) -> bool {
        self.drifting
    }

    /// the colors of each part, in part order
    pub fn fills(&self, config: &SurfaceConfig, bars: usize, order: ByteOrder) -> Vec<Fill> {
        self.parts
            .iter()
            .map(|part| part.shape.fill(config, self.size, bars, order))
            .collect()
    }

    /// the bar value `element` shows
    pub fn bar(&self, element: usize) -> usize {
        self.find(element)
            .map_or(0, |(_, part, local)| part.values.start + local)
    }

    /// `element` at `value` in `0.0..=1.0`, lifted `lift` from its edge
    pub fn pose(&self, element: usize, value: f32, lift: f32) -> Pose {
        self.find(element)
            .map_or(Pose::default(), |(_, part, local)| {
                part.shape.pose(local, value, lift)
            })
    }

    /// every pixel of `element` at `pose`
    pub fn area(&self, element: usize, pose: Pose) -> Option<PixelRect> {
        let (_, part, local) = self.find(element)?;
        part.shape.area(local, pose)
    }

    /// the pixels that differ between `element` at `old` and at `new`
    pub fn change(&self, element: usize, old: Pose, new: Pose) -> Option<PixelRect> {
        let (_, part, local) = self.find(element)?;
        part.shape.change(local, old, new)
    }

    /// every pixel of `element` at either pose
    pub fn cover(&self, element: usize, a: Pose, b: Pose) -> Option<PixelRect> {
        let (_, part, local) = self.find(element)?;
        part.shape.cover(local, a, b)
    }

    pub fn paint(
        &self,
        canvas: &mut Canvas<'_>,
        element: usize,
        pose: Pose,
        clip: PixelRect,
        fills: &[Fill],
    ) {
        let Some((index, part, local)) = self.find(element) else {
            return;
        };
        if let Some(fill) = fills.get(index) {
            let color = part.values.start + local;
            part.shape.paint(canvas, local, color, pose, clip, fill);
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
    matches!(
        layout,
        Layout::Line | Layout::Mirror | Layout::Frame | Layout::Particle | Layout::Floating
    )
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
