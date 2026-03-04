use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use kwybars_common::config::{OverlayConfig, OverlayPosition};

pub fn apply_default_size(window: &gtk::ApplicationWindow, position: &OverlayPosition) {
    match position {
        OverlayPosition::Bottom | OverlayPosition::Top => {
            window.set_default_size(800, super::HORIZONTAL_THICKNESS);
        }
        OverlayPosition::Left | OverlayPosition::Right => {
            window.set_default_size(super::VERTICAL_THICKNESS, 800);
        }
    }
}

pub fn configure_layer_shell(window: &gtk::ApplicationWindow, overlay: &OverlayConfig) {
    if !gtk4_layer_shell::is_supported() {
        eprintln!("kwybars: layer-shell is not supported by this compositor/session");
        return;
    }

    window.init_layer_shell();
    window.set_namespace(Some("kwybars"));
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_exclusive_zone(0);

    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, false);
        window.set_margin(edge, 0);
    }

    let margin = overlay.anchor_margin.min(i32::MAX as u32) as i32;
    match overlay.position {
        OverlayPosition::Bottom => {
            window.set_anchor(Edge::Left, true);
            window.set_anchor(Edge::Right, true);
            window.set_anchor(Edge::Bottom, true);
            window.set_margin(Edge::Bottom, margin);
        }
        OverlayPosition::Top => {
            window.set_anchor(Edge::Left, true);
            window.set_anchor(Edge::Right, true);
            window.set_anchor(Edge::Top, true);
            window.set_margin(Edge::Top, margin);
        }
        OverlayPosition::Left => {
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Bottom, true);
            window.set_anchor(Edge::Left, true);
            window.set_margin(Edge::Left, margin);
        }
        OverlayPosition::Right => {
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Bottom, true);
            window.set_anchor(Edge::Right, true);
            window.set_margin(Edge::Right, margin);
        }
    }
}
