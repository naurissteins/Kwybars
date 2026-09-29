//! layer-shell state from the surface placement

use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer as ShellLayer, LayerSurface,
};

use crate::config::Layer;
use crate::wayland::placement::{Anchors, Placement};

/// sends the layer state, which an unmap resets
pub(super) fn place(layer: &LayerSurface, placement: &Placement) {
    layer.set_anchor(anchor(placement.anchors));
    let margins = placement.margins;
    layer.set_margin(margins.top, margins.right, margins.bottom, margins.left);
    layer.set_size(placement.width, placement.height);
    layer.set_exclusive_zone(0);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
}

pub(super) fn shell_layer(layer: Layer) -> ShellLayer {
    match layer {
        Layer::Background => ShellLayer::Background,
        Layer::Bottom => ShellLayer::Bottom,
        Layer::Top => ShellLayer::Top,
    }
}

fn anchor(anchors: Anchors) -> Anchor {
    let mut anchor = Anchor::empty();
    anchor.set(Anchor::TOP, anchors.top);
    anchor.set(Anchor::BOTTOM, anchors.bottom);
    anchor.set(Anchor::LEFT, anchors.left);
    anchor.set(Anchor::RIGHT, anchors.right);
    anchor
}
