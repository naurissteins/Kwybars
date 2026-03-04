use gtk::gdk;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use kwybars_common::config::{
    HorizontalAlignment, OverlayConfig, OverlayPosition, VerticalAlignment,
};

pub fn apply_default_size(window: &gtk::ApplicationWindow, overlay: &OverlayConfig) {
    let width = overlay.width.max(1).min(i32::MAX as u32) as i32;
    let height = overlay.height.max(1).min(i32::MAX as u32) as i32;
    let full_extent = monitor_extent(&overlay.position);

    let (width, height) = match overlay.position {
        OverlayPosition::Bottom | OverlayPosition::Top => {
            if overlay.full_length {
                (full_extent.unwrap_or(width), height)
            } else {
                (width, height)
            }
        }
        OverlayPosition::Left | OverlayPosition::Right => {
            if overlay.full_length {
                (width, full_extent.unwrap_or(height))
            } else {
                (width, height)
            }
        }
    };

    window.set_default_size(width, height);
}

fn monitor_extent(position: &OverlayPosition) -> Option<i32> {
    let display = gdk::Display::default()?;
    let monitors = display.monitors();
    let monitor = monitors.item(0)?.downcast::<gdk::Monitor>().ok()?;
    let geometry = monitor.geometry();

    match position {
        OverlayPosition::Bottom | OverlayPosition::Top => Some(geometry.width().max(1)),
        OverlayPosition::Left | OverlayPosition::Right => Some(geometry.height().max(1)),
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
            window.set_anchor(Edge::Bottom, true);
            window.set_margin(Edge::Bottom, margin);
            apply_horizontal_span_anchors(window, overlay);
        }
        OverlayPosition::Top => {
            window.set_anchor(Edge::Top, true);
            window.set_margin(Edge::Top, margin);
            apply_horizontal_span_anchors(window, overlay);
        }
        OverlayPosition::Left => {
            window.set_anchor(Edge::Left, true);
            window.set_margin(Edge::Left, margin);
            apply_vertical_span_anchors(window, overlay);
        }
        OverlayPosition::Right => {
            window.set_anchor(Edge::Right, true);
            window.set_margin(Edge::Right, margin);
            apply_vertical_span_anchors(window, overlay);
        }
    }
}

fn apply_horizontal_span_anchors(window: &gtk::ApplicationWindow, overlay: &OverlayConfig) {
    if overlay.full_length {
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        return;
    }

    match overlay.horizontal_alignment {
        HorizontalAlignment::Left => window.set_anchor(Edge::Left, true),
        HorizontalAlignment::Center => {}
        HorizontalAlignment::Right => window.set_anchor(Edge::Right, true),
    }
}

fn apply_vertical_span_anchors(window: &gtk::ApplicationWindow, overlay: &OverlayConfig) {
    if overlay.full_length {
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        return;
    }

    match overlay.vertical_alignment {
        VerticalAlignment::Top => window.set_anchor(Edge::Top, true),
        VerticalAlignment::Center => {}
        VerticalAlignment::Bottom => window.set_anchor(Edge::Bottom, true),
    }
}
