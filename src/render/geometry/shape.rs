//! what one part of a layout draws: a strip of bars, a row of dots, or a
//! ring of turned bars

use crate::config::{GradientDirection, SurfaceConfig};
use crate::render::dots::DotLayout;
use crate::render::fill::{Axis, Fill};
use crate::render::line::LineLayout;
use crate::render::radial::RadialLayout;
use crate::render::{ByteOrder, Canvas, PixelRect, Pose};

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// bars colored along the axis
    Strip(LineLayout, Axis),
    Dots(DotLayout),
    Radial(RadialLayout),
}

impl Shape {
    pub fn fill(
        &self,
        config: &SurfaceConfig,
        size: (u32, u32),
        bars: usize,
        order: ByteOrder,
    ) -> Fill {
        match self {
            Self::Strip(_, axis) => Fill::new(config, *axis, size, bars, order),
            Self::Dots(_) => Fill::per_bar(config, bars, order, false),
            Self::Radial(_) => {
                let smooth = config.visualizer.gradient_direction == GradientDirection::Horizontal;
                Fill::per_bar(config, bars, order, smooth)
            }
        }
    }

    /// element index at bar value, lifted lift from its edge or turned by turn
    pub fn pose(&self, index: usize, value: f32, lift: f32, turn: f32) -> Pose {
        match self {
            Self::Strip(layout, _) => Pose {
                extent: layout.extent(value),
                shift: 0.0,
            },
            Self::Dots(dots) => dots.pose(index, value, lift),
            Self::Radial(radial) => radial.pose(value, turn),
        }
    }

    pub fn area(&self, index: usize, pose: Pose) -> Option<PixelRect> {
        match self {
            Self::Strip(layout, _) => layout.area(index, pose.extent),
            Self::Dots(dots) => dots.area(index, pose),
            Self::Radial(radial) => radial.area(index, pose),
        }
    }

    pub fn change(&self, index: usize, old: Pose, new: Pose) -> Option<PixelRect> {
        match self {
            Self::Strip(layout, _) => layout.change(index, old.extent, new.extent),
            Self::Dots(_) if old == new => None,
            Self::Dots(dots) => dots.cover(index, old, new),
            Self::Radial(radial) => radial.change(index, old, new),
        }
    }

    pub fn cover(&self, index: usize, a: Pose, b: Pose) -> Option<PixelRect> {
        match self {
            Self::Strip(layout, _) => layout.area(index, a.extent.max(b.extent)),
            Self::Dots(dots) => dots.cover(index, a, b),
            Self::Radial(radial) => radial.cover(index, a, b),
        }
    }

    /// about how many pixels element index touches at pose
    pub fn footprint(&self, index: usize, pose: Pose) -> u64 {
        match self {
            Self::Radial(radial) => radial.footprint(pose),
            _ => self.area(index, pose).map_or(0, PixelRect::pixels),
        }
    }

    /// makes every pixel of element index at pose transparent
    pub fn clear(&self, canvas: &mut Canvas<'_>, index: usize, pose: Pose) {
        match self {
            Self::Radial(radial) => radial.clear(canvas, index, pose),
            _ => {
                if let Some(area) = self.area(index, pose) {
                    canvas.clear(area);
                }
            }
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
            Self::Radial(radial) => radial.paint(canvas, index, color, pose, clip, fill),
        }
    }
}
