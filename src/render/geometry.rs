//! the layout a surface draws, chosen from its layout key

use super::fill::{Axis, Fill};
use super::line::LineLayout;
use super::{Canvas, PixelRect, mirror};
use crate::config::{Config, Layout, SurfaceConfig};

/// bar geometry for one buffer size, in buffer pixels: one or more strips
/// of bars, each drawing every bar
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    size: (u32, u32),
    bars: usize,
    strips: Vec<LineLayout>,
    axis: Axis,
}

impl Geometry {
    /// lays out bars bars for the surface's layout, layouts not ported yet
    /// are drawn as line
    pub fn new(config: &SurfaceConfig, size: (u32, u32), scale: f32, bars: usize) -> Self {
        let visualizer = &config.visualizer;
        let (strips, axis) = match visualizer.layout {
            Layout::Mirror => {
                let (halves, axis) = mirror::strips(config, size, scale);
                let strips = halves
                    .into_iter()
                    .map(|strip| LineLayout::new(visualizer, strip, size, scale, bars))
                    .collect();
                (strips, axis)
            }
            Layout::Line
            | Layout::Wave
            | Layout::Frame
            | Layout::Radial
            | Layout::Polygon
            | Layout::Particle
            | Layout::Floating => {
                let edge = config.overlay.position;
                let line = LineLayout::along(visualizer, edge, size, scale, bars);
                let axis = Axis::along(edge, visualizer.gradient_direction, size);
                (vec![line], axis)
            }
        };
        Self {
            size,
            bars,
            strips,
            axis,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn elements(&self) -> usize {
        self.strips.len() * self.bars
    }

    /// the bar element `element` shows
    pub fn bar(&self, element: usize) -> usize {
        element.checked_rem(self.bars).unwrap_or(0)
    }

    /// the line gradients run along
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// whether the drawing changes over time with the bars at rest, such as
    /// a rotation, so shown surfaces keep getting frames
    pub fn animates(&self) -> bool {
        false
    }

    /// how far element `element` reaches at `value` in `0.0..=1.0`, in pixels
    pub fn extent(&self, element: usize, value: f32) -> f32 {
        self.strip(element)
            .map_or(0.0, |(strip, _)| strip.extent(value))
    }

    /// every pixel of element at extent
    pub fn area(&self, element: usize, extent: f32) -> Option<PixelRect> {
        let (strip, bar) = self.strip(element)?;
        strip.area(bar, extent)
    }

    /// the pixels that differ between element at old and at new
    pub fn change(&self, element: usize, old: f32, new: f32) -> Option<PixelRect> {
        let (strip, bar) = self.strip(element)?;
        strip.change(bar, old, new)
    }

    /// draws element at extent inside clip, which must be clear
    pub fn paint(
        &self,
        canvas: &mut Canvas<'_>,
        element: usize,
        extent: f32,
        clip: PixelRect,
        fill: &Fill,
    ) {
        if let Some((strip, bar)) = self.strip(element) {
            strip.paint(canvas, bar, extent, clip, fill);
        }
    }

    fn strip(&self, element: usize) -> Option<(&LineLayout, usize)> {
        let strip = self.strips.get(element.checked_div(self.bars)?)?;
        Some((strip, self.bar(element)))
    }
}

/// whether layout has its own drawing yet
pub fn is_ported(layout: Layout) -> bool {
    matches!(layout, Layout::Line | Layout::Mirror)
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
