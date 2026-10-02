//! where a layer surface goes: anchors, margins, and requested size

#[cfg(test)]
mod tests;

use crate::config::{Edge, HorizontalAlignment, Layer, Layout, OverlaySettings, VerticalAlignment};

/// screen edges a surface is attached to
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Anchors {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

/// distance from each anchored edge, in logical pixels
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Margins {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

/// what a layer surface asks the compositor for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub layer: Layer,
    pub anchors: Anchors,
    pub margins: Margins,
    /// logical width; 0 stretches between the left and right anchors
    pub width: u32,
    /// logical height; 0 stretches between the top and bottom anchors
    pub height: u32,
}

impl Placement {
    /// placement for one surface, following the legacy overlay window rules
    pub fn new(overlay: &OverlaySettings, layout: Layout) -> Self {
        let margin = |value: u32| i32::try_from(value).unwrap_or(i32::MAX);
        let mut anchors = Anchors::default();
        let mut margins = Margins::default();

        if fills_output(layout) {
            anchors = Anchors {
                top: true,
                bottom: true,
                left: true,
                right: true,
            };
            margins = Margins {
                top: margin(overlay.margin_top),
                right: margin(overlay.margin_right),
                bottom: margin(overlay.margin_bottom),
                left: margin(overlay.margin_left),
            };
            return Self {
                layer: overlay.layer,
                anchors,
                margins,
                width: 0,
                height: 0,
            };
        }

        let edge_margin = margin(overlay.anchor_margin);
        let horizontal = matches!(overlay.position, Edge::Bottom | Edge::Top);
        match overlay.position {
            Edge::Bottom => (anchors.bottom, margins.bottom) = (true, edge_margin),
            Edge::Top => (anchors.top, margins.top) = (true, edge_margin),
            Edge::Left => (anchors.left, margins.left) = (true, edge_margin),
            Edge::Right => (anchors.right, margins.right) = (true, edge_margin),
        }

        if horizontal {
            let (left, right) = match (overlay.full_length, overlay.horizontal_alignment) {
                (true, _) => (true, true),
                (false, HorizontalAlignment::Left) => (true, false),
                (false, HorizontalAlignment::Center) => (false, false),
                (false, HorizontalAlignment::Right) => (false, true),
            };
            (anchors.left, anchors.right) = (left, right);
            margins.left = if left { margin(overlay.margin_left) } else { 0 };
            margins.right = if right {
                margin(overlay.margin_right)
            } else {
                0
            };
        } else {
            let (top, bottom) = match (overlay.full_length, overlay.vertical_alignment) {
                (true, _) => (true, true),
                (false, VerticalAlignment::Top) => (true, false),
                (false, VerticalAlignment::Center) => (false, false),
                (false, VerticalAlignment::Bottom) => (false, true),
            };
            (anchors.top, anchors.bottom) = (top, bottom);
            margins.top = if top { margin(overlay.margin_top) } else { 0 };
            margins.bottom = if bottom {
                margin(overlay.margin_bottom)
            } else {
                0
            };
        }

        let stretch = overlay.full_length;
        let width = if horizontal && stretch {
            0
        } else {
            overlay.width.max(1)
        };
        let height = if !horizontal && stretch {
            0
        } else {
            overlay.height.max(1)
        };
        Self {
            layer: overlay.layer,
            anchors,
            margins,
            width,
            height,
        }
    }

    /// the logical size to draw at after a configure
    pub fn surface_size(&self, configured: (u32, u32), output: Option<(u32, u32)>) -> (u32, u32) {
        let pick =
            |configured: u32, requested: u32, output: Option<u32>, before: i32, after: i32| {
                if configured > 0 {
                    return configured;
                }
                if requested > 0 {
                    return requested;
                }
                let margins = i64::from(before) + i64::from(after);
                output.map_or(1, |extent| {
                    u32::try_from((i64::from(extent) - margins).max(1)).unwrap_or(1)
                })
            };
        (
            pick(
                configured.0,
                self.width,
                output.map(|size| size.0),
                self.margins.left,
                self.margins.right,
            ),
            pick(
                configured.1,
                self.height,
                output.map(|size| size.1),
                self.margins.top,
                self.margins.bottom,
            ),
        )
    }
}

/// layouts drawn over the whole output instead of along one edge
fn fills_output(layout: Layout) -> bool {
    matches!(
        layout,
        Layout::Mirror | Layout::Frame | Layout::Radial | Layout::Polygon
    )
}
