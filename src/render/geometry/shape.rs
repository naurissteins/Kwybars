//! what one part of a layout draws: a strip of bars or a row of dots

use crate::config::SurfaceConfig;
use crate::render::dots::DotLayout;
use crate::render::fill::{Axis, Fill};
use crate::render::line::LineLayout;
use crate::render::{ByteOrder, Canvas, PixelRect, Pose};

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// bars colored along `Axis`
    Strip(LineLayout, Axis),
    Dots(DotLayout),
}

impl Shape {
    /// the colors of this part in a buffer of `size`
    pub fn fill(
        &self,
        config: &SurfaceConfig,
        size: (u32, u32),
        bars: usize,
        order: ByteOrder,
    ) -> Fill {
        match self {
            Self::Strip(_, axis) => Fill::new(config, *axis, size, bars, order),
            Self::Dots(_) => Fill::per_bar(config, bars, order),
        }
    }

    /// element `index` at bar `value` in `0.0..=1.0`, lifted `lift` from its edge
    pub fn pose(&self, index: usize, value: f32, lift: f32) -> Pose {
        match self {
            Self::Strip(layout, _) => Pose {
                extent: layout.extent(value),
                shift: 0.0,
            },
            Self::Dots(dots) => dots.pose(index, value, lift),
        }
    }

    pub fn area(&self, index: usize, pose: Pose) -> Option<PixelRect> {
        match self {
            Self::Strip(layout, _) => layout.area(index, pose.extent),
            Self::Dots(dots) => dots.area(index, pose),
        }
    }

    pub fn change(&self, index: usize, old: Pose, new: Pose) -> Option<PixelRect> {
        match self {
            Self::Strip(layout, _) => layout.change(index, old.extent, new.extent),
            Self::Dots(_) if old == new => None,
            Self::Dots(dots) => dots.cover(index, old, new),
        }
    }

    pub fn cover(&self, index: usize, a: Pose, b: Pose) -> Option<PixelRect> {
        match self {
            Self::Strip(layout, _) => layout.area(index, a.extent.max(b.extent)),
            Self::Dots(dots) => dots.cover(index, a, b),
        }
    }

    pub fn paint(
        &self,
        canvas: &mut Canvas<'_>,
        index: usize,
        color: usize,
        pose: Pose,
        clip: PixelRect,
        fill: &Fill,
    ) {
        match self {
            Self::Strip(layout, _) => layout.paint(canvas, index, color, pose.extent, clip, fill),
            Self::Dots(dots) => dots.paint(canvas, index, color, pose, clip, fill),
        }
    }
}
