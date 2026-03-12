use gtk::gdk;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use kwybars_common::config::{
    AppConfig, HorizontalAlignment, OverlayConfig, OverlayLayer, OverlayMonitorMode,
    OverlayPosition, VerticalAlignment, VisualizerLayout,
};
use tracing::warn;

pub fn selected_monitors(overlay: &OverlayConfig) -> Vec<gdk::Monitor> {
    let Some(display) = gdk::Display::default() else {
        return Vec::new();
    };

    let monitors_model = display.monitors();
    let monitors: Vec<gdk::Monitor> = (0..monitors_model.n_items())
        .filter_map(|index| monitors_model.item(index))
        .filter_map(|item| item.downcast::<gdk::Monitor>().ok())
        .collect();

    if monitors.is_empty() {
        return Vec::new();
    }

    match overlay.monitor_mode {
        OverlayMonitorMode::Primary => vec![monitors[0].clone()],
        OverlayMonitorMode::All => monitors,
        OverlayMonitorMode::List => resolve_named_monitors(monitors, &overlay.monitors),
    }
}

fn resolve_named_monitors(monitors: Vec<gdk::Monitor>, requested: &[String]) -> Vec<gdk::Monitor> {
    if requested.is_empty() {
        return vec![monitors[0].clone()];
    }

    let mut used_indices = std::collections::BTreeSet::new();
    let mut selected = Vec::new();

    for requested_name in requested {
        let requested_name = requested_name.trim();
        if requested_name.is_empty() {
            continue;
        }

        if let Some(index) = parse_monitor_index(requested_name, monitors.len()) {
            if used_indices.insert(index) {
                selected.push(monitors[index].clone());
            }
            continue;
        }

        let Some(index) = monitors.iter().enumerate().find_map(|(index, monitor)| {
            let connector = monitor.connector()?;
            if connector.as_str() == requested_name {
                Some(index)
            } else {
                None
            }
        }) else {
            continue;
        };

        if used_indices.insert(index) {
            selected.push(monitors[index].clone());
        }
    }

    if selected.is_empty() {
        vec![monitors[0].clone()]
    } else {
        selected
    }
}

fn parse_monitor_index(raw: &str, max: usize) -> Option<usize> {
    let one_based = if let Some(rest) = raw.strip_prefix("index:") {
        rest.parse::<usize>().ok()?
    } else {
        raw.parse::<usize>().ok()?
    };

    if one_based == 0 {
        return None;
    }

    let index = one_based - 1;
    if index < max { Some(index) } else { None }
}

pub fn apply_default_size(
    window: &gtk::ApplicationWindow,
    config: &AppConfig,
    monitor: Option<&gdk::Monitor>,
) {
    let overlay = &config.overlay;
    if uses_centered_layout(config.visualizer.layout) {
        let (width, height) = centered_window_size(overlay, monitor);
        window.set_default_size(width, height);
        return;
    }

    let width = overlay.width.max(1).min(i32::MAX as u32) as i32;
    let height = overlay.height.max(1).min(i32::MAX as u32) as i32;
    let full_extent = monitor_extent(&overlay.position, monitor);
    let margin_left = to_margin_i32(overlay.margin_left);
    let margin_right = to_margin_i32(overlay.margin_right);
    let margin_top = to_margin_i32(overlay.margin_top);
    let margin_bottom = to_margin_i32(overlay.margin_bottom);

    let (width, height) = match overlay.position {
        OverlayPosition::Bottom | OverlayPosition::Top => {
            if overlay.full_length {
                (
                    shrunk_extent(full_extent.unwrap_or(width), margin_left, margin_right),
                    height,
                )
            } else {
                (width, height)
            }
        }
        OverlayPosition::Left | OverlayPosition::Right => {
            if overlay.full_length {
                (
                    width,
                    shrunk_extent(full_extent.unwrap_or(height), margin_top, margin_bottom),
                )
            } else {
                (width, height)
            }
        }
    };

    window.set_default_size(width, height);
}

fn monitor_extent(position: &OverlayPosition, monitor: Option<&gdk::Monitor>) -> Option<i32> {
    let monitor = monitor.cloned().or_else(|| {
        let display = gdk::Display::default()?;
        let monitors = display.monitors();
        monitors.item(0)?.downcast::<gdk::Monitor>().ok()
    })?;
    let geometry = monitor.geometry();

    match position {
        OverlayPosition::Bottom | OverlayPosition::Top => Some(geometry.width().max(1)),
        OverlayPosition::Left | OverlayPosition::Right => Some(geometry.height().max(1)),
    }
}

pub fn configure_layer_shell(
    window: &gtk::ApplicationWindow,
    config: &AppConfig,
    monitor: Option<&gdk::Monitor>,
) {
    let overlay = &config.overlay;
    if !gtk4_layer_shell::is_supported() {
        warn!("kwybars: layer-shell is not supported by this compositor/session");
        return;
    }

    window.init_layer_shell();
    window.set_monitor(monitor);
    window.set_namespace(Some("kwybars"));
    window.set_layer(match overlay.layer {
        OverlayLayer::Background => Layer::Background,
        OverlayLayer::Bottom => Layer::Bottom,
        OverlayLayer::Top => Layer::Top,
    });
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_exclusive_zone(0);

    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, false);
        window.set_margin(edge, 0);
    }

    if uses_centered_layout(config.visualizer.layout) {
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Top, to_margin_i32(overlay.margin_top));
        window.set_margin(Edge::Bottom, to_margin_i32(overlay.margin_bottom));
        window.set_margin(Edge::Left, to_margin_i32(overlay.margin_left));
        window.set_margin(Edge::Right, to_margin_i32(overlay.margin_right));
        return;
    }

    let primary_margin = to_margin_i32(overlay.anchor_margin);
    match overlay.position {
        OverlayPosition::Bottom => {
            window.set_anchor(Edge::Bottom, true);
            window.set_margin(Edge::Bottom, primary_margin);
            apply_horizontal_span_anchors(window, overlay);
        }
        OverlayPosition::Top => {
            window.set_anchor(Edge::Top, true);
            window.set_margin(Edge::Top, primary_margin);
            apply_horizontal_span_anchors(window, overlay);
        }
        OverlayPosition::Left => {
            window.set_anchor(Edge::Left, true);
            window.set_margin(Edge::Left, primary_margin);
            apply_vertical_span_anchors(window, overlay);
        }
        OverlayPosition::Right => {
            window.set_anchor(Edge::Right, true);
            window.set_margin(Edge::Right, primary_margin);
            apply_vertical_span_anchors(window, overlay);
        }
    }
}

fn apply_horizontal_span_anchors(window: &gtk::ApplicationWindow, overlay: &OverlayConfig) {
    let margin_left = to_margin_i32(overlay.margin_left);
    let margin_right = to_margin_i32(overlay.margin_right);

    if overlay.full_length {
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Left, margin_left);
        window.set_margin(Edge::Right, margin_right);
        return;
    }

    match overlay.horizontal_alignment {
        HorizontalAlignment::Left => {
            window.set_anchor(Edge::Left, true);
            window.set_margin(Edge::Left, margin_left);
        }
        HorizontalAlignment::Center => {}
        HorizontalAlignment::Right => {
            window.set_anchor(Edge::Right, true);
            window.set_margin(Edge::Right, margin_right);
        }
    }
}

fn apply_vertical_span_anchors(window: &gtk::ApplicationWindow, overlay: &OverlayConfig) {
    let margin_top = to_margin_i32(overlay.margin_top);
    let margin_bottom = to_margin_i32(overlay.margin_bottom);

    if overlay.full_length {
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_margin(Edge::Top, margin_top);
        window.set_margin(Edge::Bottom, margin_bottom);
        return;
    }

    match overlay.vertical_alignment {
        VerticalAlignment::Top => {
            window.set_anchor(Edge::Top, true);
            window.set_margin(Edge::Top, margin_top);
        }
        VerticalAlignment::Center => {}
        VerticalAlignment::Bottom => {
            window.set_anchor(Edge::Bottom, true);
            window.set_margin(Edge::Bottom, margin_bottom);
        }
    }
}

fn to_margin_i32(value: u32) -> i32 {
    value.min(i32::MAX as u32) as i32
}

fn centered_window_size(overlay: &OverlayConfig, monitor: Option<&gdk::Monitor>) -> (i32, i32) {
    let fallback_width = overlay.width.max(1).min(i32::MAX as u32) as i32;
    let fallback_height = overlay.height.max(1).min(i32::MAX as u32) as i32;
    let Some(geometry) = monitor_geometry(monitor) else {
        return (fallback_width, fallback_height);
    };

    let width = shrunk_extent(
        geometry.width().max(1),
        to_margin_i32(overlay.margin_left),
        to_margin_i32(overlay.margin_right),
    );
    let height = shrunk_extent(
        geometry.height().max(1),
        to_margin_i32(overlay.margin_top),
        to_margin_i32(overlay.margin_bottom),
    );

    (width, height)
}

fn uses_centered_layout(layout: VisualizerLayout) -> bool {
    matches!(
        layout,
        VisualizerLayout::Frame | VisualizerLayout::Radial | VisualizerLayout::Polygon
    )
}

fn monitor_geometry(monitor: Option<&gdk::Monitor>) -> Option<gdk::Rectangle> {
    let monitor = monitor.cloned().or_else(|| {
        let display = gdk::Display::default()?;
        let monitors = display.monitors();
        monitors.item(0)?.downcast::<gdk::Monitor>().ok()
    })?;

    Some(monitor.geometry())
}

fn shrunk_extent(extent: i32, before: i32, after: i32) -> i32 {
    let total_margin = before.saturating_add(after);
    extent.saturating_sub(total_margin).max(1)
}
