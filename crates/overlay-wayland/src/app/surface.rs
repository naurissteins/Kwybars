use kwybars_common::config::{
    AppConfig, HorizontalAlignment, OverlayLayer, OverlayPosition, VerticalAlignment,
    VisualizerLayout,
};
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

const FALLBACK_EXPANDED_DIMENSION: u32 = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceMargins {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl SurfaceMargins {
    fn zero() -> Self {
        Self {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceConfig {
    pub layer: Layer,
    pub anchor: Anchor,
    pub margins: SurfaceMargins,
    pub requested_width: u32,
    pub requested_height: u32,
    pub fallback_width: u32,
    pub fallback_height: u32,
}

impl SurfaceConfig {
    pub fn from_app_config(config: &AppConfig) -> Self {
        let overlay = &config.overlay;
        if uses_centered_layout(config.visualizer.layout) {
            return Self {
                layer: map_layer(overlay.layer.clone()),
                anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
                margins: SurfaceMargins {
                    top: to_margin_i32(overlay.margin_top),
                    right: to_margin_i32(overlay.margin_right),
                    bottom: to_margin_i32(overlay.margin_bottom),
                    left: to_margin_i32(overlay.margin_left),
                },
                requested_width: 0,
                requested_height: 0,
                fallback_width: overlay.width.max(1),
                fallback_height: overlay.height.max(1),
            };
        }

        let mut anchor = Anchor::empty();
        let mut margins = SurfaceMargins::zero();
        let primary_margin = to_margin_i32(overlay.anchor_margin);

        match overlay.position {
            OverlayPosition::Bottom => {
                anchor |= Anchor::BOTTOM;
                margins.bottom = primary_margin;
                apply_horizontal_span(overlay, &mut anchor, &mut margins);
            }
            OverlayPosition::Top => {
                anchor |= Anchor::TOP;
                margins.top = primary_margin;
                apply_horizontal_span(overlay, &mut anchor, &mut margins);
            }
            OverlayPosition::Left => {
                anchor |= Anchor::LEFT;
                margins.left = primary_margin;
                apply_vertical_span(overlay, &mut anchor, &mut margins);
            }
            OverlayPosition::Right => {
                anchor |= Anchor::RIGHT;
                margins.right = primary_margin;
                apply_vertical_span(overlay, &mut anchor, &mut margins);
            }
        }

        let requested_width = match overlay.position {
            OverlayPosition::Bottom | OverlayPosition::Top => {
                if overlay.full_length {
                    0
                } else {
                    overlay.width.max(1)
                }
            }
            OverlayPosition::Left | OverlayPosition::Right => overlay.width.max(1),
        };
        let requested_height = match overlay.position {
            OverlayPosition::Bottom | OverlayPosition::Top => overlay.height.max(1),
            OverlayPosition::Left | OverlayPosition::Right => {
                if overlay.full_length {
                    0
                } else {
                    overlay.height.max(1)
                }
            }
        };

        Self {
            layer: map_layer(overlay.layer.clone()),
            anchor,
            margins,
            requested_width,
            requested_height,
            fallback_width: fallback_dimension(requested_width, overlay.width.max(1)),
            fallback_height: fallback_dimension(requested_height, overlay.height.max(1)),
        }
    }

    pub fn resolved_dimensions(
        &self,
        configured_width: u32,
        configured_height: u32,
        output_size: Option<(i32, i32)>,
    ) -> (u32, u32) {
        let width = if configured_width == 0 {
            self.expanded_or_fallback_width(output_size)
        } else {
            configured_width
        };
        let height = if configured_height == 0 {
            self.expanded_or_fallback_height(output_size)
        } else {
            configured_height
        };

        (width.max(1), height.max(1))
    }

    fn expanded_or_fallback_width(&self, output_size: Option<(i32, i32)>) -> u32 {
        if self.requested_width != 0 {
            return self.fallback_width;
        }

        output_size
            .map(|(width, _)| shrink_extent(width, self.margins.left, self.margins.right))
            .unwrap_or(self.fallback_width)
    }

    fn expanded_or_fallback_height(&self, output_size: Option<(i32, i32)>) -> u32 {
        if self.requested_height != 0 {
            return self.fallback_height;
        }

        output_size
            .map(|(_, height)| shrink_extent(height, self.margins.top, self.margins.bottom))
            .unwrap_or(self.fallback_height)
    }
}

fn map_layer(layer: OverlayLayer) -> Layer {
    match layer {
        OverlayLayer::Background => Layer::Background,
        OverlayLayer::Bottom => Layer::Bottom,
        OverlayLayer::Top => Layer::Top,
    }
}

fn apply_horizontal_span(
    overlay: &kwybars_common::config::OverlayConfig,
    anchor: &mut Anchor,
    margins: &mut SurfaceMargins,
) {
    if overlay.full_length {
        *anchor |= Anchor::LEFT | Anchor::RIGHT;
        margins.left = to_margin_i32(overlay.margin_left);
        margins.right = to_margin_i32(overlay.margin_right);
        return;
    }

    match overlay.horizontal_alignment {
        HorizontalAlignment::Left => {
            *anchor |= Anchor::LEFT;
            margins.left = to_margin_i32(overlay.margin_left);
        }
        HorizontalAlignment::Center => {}
        HorizontalAlignment::Right => {
            *anchor |= Anchor::RIGHT;
            margins.right = to_margin_i32(overlay.margin_right);
        }
    }
}

fn apply_vertical_span(
    overlay: &kwybars_common::config::OverlayConfig,
    anchor: &mut Anchor,
    margins: &mut SurfaceMargins,
) {
    if overlay.full_length {
        *anchor |= Anchor::TOP | Anchor::BOTTOM;
        margins.top = to_margin_i32(overlay.margin_top);
        margins.bottom = to_margin_i32(overlay.margin_bottom);
        return;
    }

    match overlay.vertical_alignment {
        VerticalAlignment::Top => {
            *anchor |= Anchor::TOP;
            margins.top = to_margin_i32(overlay.margin_top);
        }
        VerticalAlignment::Center => {}
        VerticalAlignment::Bottom => {
            *anchor |= Anchor::BOTTOM;
            margins.bottom = to_margin_i32(overlay.margin_bottom);
        }
    }
}

fn fallback_dimension(requested: u32, explicit: u32) -> u32 {
    if requested == 0 {
        FALLBACK_EXPANDED_DIMENSION.max(explicit)
    } else {
        requested
    }
}

fn shrink_extent(extent: i32, before: i32, after: i32) -> u32 {
    extent
        .saturating_sub(before.saturating_add(after))
        .clamp(1, i32::MAX) as u32
}

fn to_margin_i32(value: u32) -> i32 {
    value.min(i32::MAX as u32) as i32
}

fn uses_centered_layout(layout: VisualizerLayout) -> bool {
    matches!(
        layout,
        VisualizerLayout::Mirror
            | VisualizerLayout::Frame
            | VisualizerLayout::Radial
            | VisualizerLayout::Polygon
    )
}

#[cfg(test)]
mod tests {
    use kwybars_common::config::{
        AppConfig, HorizontalAlignment, OverlayLayer, OverlayPosition, VerticalAlignment,
        VisualizerLayout,
    };
    use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

    use super::{SurfaceConfig, SurfaceMargins};

    #[test]
    fn bottom_full_length_uses_bottom_edge_and_horizontal_span() {
        let config = AppConfig::default();

        let surface = SurfaceConfig::from_app_config(&config);

        assert_eq!(surface.layer, Layer::Background);
        assert_eq!(
            surface.anchor,
            Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT
        );
        assert_eq!(
            surface.margins,
            SurfaceMargins {
                top: 0,
                right: 20,
                bottom: 20,
                left: 20,
            }
        );
        assert_eq!(surface.requested_width, 0);
        assert_eq!(surface.requested_height, 500);
    }

    #[test]
    fn top_right_aligned_fixed_length_uses_top_and_right_anchors() {
        let mut config = AppConfig::default();
        config.overlay.position = OverlayPosition::Top;
        config.overlay.layer = OverlayLayer::Top;
        config.overlay.full_length = false;
        config.overlay.width = 420;
        config.overlay.height = 160;
        config.overlay.horizontal_alignment = HorizontalAlignment::Right;
        config.overlay.margin_right = 32;
        config.overlay.anchor_margin = 18;

        let surface = SurfaceConfig::from_app_config(&config);

        assert_eq!(surface.layer, Layer::Top);
        assert_eq!(surface.anchor, Anchor::TOP | Anchor::RIGHT);
        assert_eq!(
            surface.margins,
            SurfaceMargins {
                top: 18,
                right: 32,
                bottom: 0,
                left: 0,
            }
        );
        assert_eq!(surface.requested_width, 420);
        assert_eq!(surface.requested_height, 160);
    }

    #[test]
    fn left_full_length_uses_vertical_span_and_explicit_thickness() {
        let mut config = AppConfig::default();
        config.overlay.position = OverlayPosition::Left;
        config.overlay.width = 72;
        config.overlay.margin_top = 12;
        config.overlay.margin_bottom = 16;

        let surface = SurfaceConfig::from_app_config(&config);

        assert_eq!(surface.anchor, Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM);
        assert_eq!(surface.requested_width, 72);
        assert_eq!(surface.requested_height, 0);
        assert_eq!(
            surface.margins,
            SurfaceMargins {
                top: 12,
                right: 0,
                bottom: 16,
                left: 20,
            }
        );
    }

    #[test]
    fn centered_layout_anchors_all_edges() {
        let mut config = AppConfig::default();
        config.visualizer.layout = VisualizerLayout::Radial;
        config.overlay.margin_top = 10;
        config.overlay.margin_right = 11;
        config.overlay.margin_bottom = 12;
        config.overlay.margin_left = 13;

        let surface = SurfaceConfig::from_app_config(&config);

        assert_eq!(
            surface.anchor,
            Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT
        );
        assert_eq!(
            surface.margins,
            SurfaceMargins {
                top: 10,
                right: 11,
                bottom: 12,
                left: 13,
            }
        );
        assert_eq!(surface.requested_width, 0);
        assert_eq!(surface.requested_height, 0);
    }

    #[test]
    fn right_centered_fixed_length_does_not_anchor_vertical_span() {
        let mut config = AppConfig::default();
        config.overlay.position = OverlayPosition::Right;
        config.overlay.full_length = false;
        config.overlay.width = 64;
        config.overlay.height = 280;
        config.overlay.vertical_alignment = VerticalAlignment::Center;

        let surface = SurfaceConfig::from_app_config(&config);

        assert_eq!(surface.anchor, Anchor::RIGHT);
        assert_eq!(surface.requested_width, 64);
        assert_eq!(surface.requested_height, 280);
    }

    #[test]
    fn resolves_vertical_full_length_from_output_height() {
        let mut config = AppConfig::default();
        config.overlay.position = OverlayPosition::Left;
        config.overlay.width = 72;
        config.overlay.margin_top = 12;
        config.overlay.margin_bottom = 16;

        let surface = SurfaceConfig::from_app_config(&config);
        let (width, height) = surface.resolved_dimensions(0, 0, Some((1920, 1080)));

        assert_eq!(width, 72);
        assert_eq!(height, 1052);
    }

    #[test]
    fn resolves_horizontal_full_length_from_output_width() {
        let mut config = AppConfig::default();
        config.overlay.position = OverlayPosition::Bottom;
        config.overlay.margin_left = 20;
        config.overlay.margin_right = 24;

        let surface = SurfaceConfig::from_app_config(&config);
        let (width, height) = surface.resolved_dimensions(0, 0, Some((1920, 1080)));

        assert_eq!(width, 1876);
        assert_eq!(height, 500);
    }
}
