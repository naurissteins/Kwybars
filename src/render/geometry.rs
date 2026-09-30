//! the layout a surface draws, chosen from its `layout` key

use super::fill::Fill;
use super::line::LineLayout;
use super::{Canvas, PixelRect};
use crate::config::{Config, Edge, Layout, VisualizerConfig};

/// bar geometry for one buffer size, in buffer pixels
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    Line(LineLayout),
}

impl Geometry {
    /// lays out `bars` bars for `visualizer.layout`; layouts not ported yet
    /// are drawn as `line`
    pub fn new(
        visualizer: &VisualizerConfig,
        edge: Edge,
        size: (u32, u32),
        scale: f32,
        bars: usize,
    ) -> Self {
        match visualizer.layout {
            Layout::Line
            | Layout::Mirror
            | Layout::Wave
            | Layout::Frame
            | Layout::Radial
            | Layout::Polygon
            | Layout::Particle
            | Layout::Floating => Self::Line(LineLayout::new(visualizer, edge, size, scale, bars)),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        match self {
            Self::Line(line) => line.size(),
        }
    }

    pub fn bars(&self) -> usize {
        match self {
            Self::Line(line) => line.bars(),
        }
    }

    /// whether the drawing changes over time with the bars at rest, such as
    /// a rotation, so shown surfaces keep getting frames
    pub fn animates(&self) -> bool {
        match self {
            Self::Line(_) => false,
        }
    }

    /// how far bar `value` in `0.0..=1.0` reaches, in pixels
    pub fn extent(&self, value: f32) -> f32 {
        match self {
            Self::Line(line) => line.extent(value),
        }
    }

    /// every pixel of bar `index` at `extent`
    pub fn area(&self, index: usize, extent: f32) -> Option<PixelRect> {
        match self {
            Self::Line(line) => line.area(index, extent),
        }
    }

    /// the pixels that differ between bar `index` at `old` and at `new`
    pub fn change(&self, index: usize, old: f32, new: f32) -> Option<PixelRect> {
        match self {
            Self::Line(line) => line.change(index, old, new),
        }
    }

    /// draws bar `index` at `extent` inside `clip`, which must be clear
    pub fn paint(
        &self,
        canvas: &mut Canvas<'_>,
        index: usize,
        extent: f32,
        clip: PixelRect,
        fill: &Fill,
    ) {
        match self {
            Self::Line(line) => line.paint(canvas, index, extent, clip, fill),
        }
    }
}

/// whether `layout` has its own drawing yet
pub fn is_ported(layout: Layout) -> bool {
    layout == Layout::Line
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
