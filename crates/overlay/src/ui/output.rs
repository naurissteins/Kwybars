use gtk::gdk;
use gtk::prelude::*;
use kwybars_common::config::{AppConfig, OverlayConfig, OverlayMonitorMode};
use tracing::warn;

pub(super) struct OverlayWindowTarget {
    pub(super) config: AppConfig,
    pub(super) monitor: Option<gdk::Monitor>,
    pub(super) use_global_theme: bool,
}

pub(super) fn window_targets(config: &AppConfig) -> Vec<OverlayWindowTarget> {
    if config.overlay.outputs.is_empty() {
        return legacy_window_targets(config);
    }

    let monitors = available_monitors();
    let mut used_indices = std::collections::BTreeSet::new();
    let mut targets = Vec::new();

    for output in &config.overlay.outputs {
        if !output.enabled {
            continue;
        }

        let Some(index) = monitor_index_for_name(&monitors, &output.monitor) else {
            warn!(
                "kwybars: configured overlay output monitor not found: {}",
                output.monitor
            );
            continue;
        };
        if !used_indices.insert(index) {
            warn!(
                "kwybars: duplicate overlay output monitor ignored: {}",
                output.monitor
            );
            continue;
        }

        let mut target_config = config.clone();
        target_config.overlay = output.merged_overlay(&config.overlay);
        target_config.visualizer = output.merged_visualizer(&config.visualizer);
        targets.push(OverlayWindowTarget {
            config: target_config,
            monitor: Some(monitors[index].clone()),
            use_global_theme: !output.visualizer.overrides_direct_colors(),
        });
    }

    targets
}

fn legacy_window_targets(config: &AppConfig) -> Vec<OverlayWindowTarget> {
    let monitors = selected_monitors(&config.overlay);
    if monitors.is_empty() {
        return vec![OverlayWindowTarget {
            config: config.clone(),
            monitor: None,
            use_global_theme: true,
        }];
    }

    monitors
        .into_iter()
        .map(|monitor| OverlayWindowTarget {
            config: config.clone(),
            monitor: Some(monitor),
            use_global_theme: true,
        })
        .collect()
}

fn selected_monitors(overlay: &OverlayConfig) -> Vec<gdk::Monitor> {
    let monitors = available_monitors();
    if monitors.is_empty() {
        return Vec::new();
    }

    match overlay.monitor_mode {
        OverlayMonitorMode::Primary => vec![monitors[0].clone()],
        OverlayMonitorMode::All => monitors,
        OverlayMonitorMode::List => resolve_named_monitors(monitors, &overlay.monitors),
    }
}

fn available_monitors() -> Vec<gdk::Monitor> {
    let Some(display) = gdk::Display::default() else {
        return Vec::new();
    };

    let monitors_model = display.monitors();
    (0..monitors_model.n_items())
        .filter_map(|index| monitors_model.item(index))
        .filter_map(|item| item.downcast::<gdk::Monitor>().ok())
        .collect()
}

fn resolve_named_monitors(monitors: Vec<gdk::Monitor>, requested: &[String]) -> Vec<gdk::Monitor> {
    if requested.is_empty() {
        return vec![monitors[0].clone()];
    }

    let mut used_indices = std::collections::BTreeSet::new();
    let mut selected = Vec::new();

    for requested_name in requested {
        let Some(index) = monitor_index_for_name(&monitors, requested_name) else {
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

fn monitor_index_for_name(monitors: &[gdk::Monitor], requested_name: &str) -> Option<usize> {
    let requested_name = requested_name.trim();
    if requested_name.is_empty() {
        return None;
    }

    if requested_name == "primary" && !monitors.is_empty() {
        return Some(0);
    }

    if let Some(index) = parse_monitor_index(requested_name, monitors.len()) {
        return Some(index);
    }

    monitors.iter().enumerate().find_map(|(index, monitor)| {
        let connector = monitor.connector()?;
        if connector.as_str() == requested_name {
            Some(index)
        } else {
            None
        }
    })
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

#[cfg(test)]
mod tests {
    use super::parse_monitor_index;

    #[test]
    fn parses_one_based_monitor_indices() {
        assert_eq!(parse_monitor_index("1", 2), Some(0));
        assert_eq!(parse_monitor_index("index:2", 2), Some(1));
        assert_eq!(parse_monitor_index("0", 2), None);
        assert_eq!(parse_monitor_index("index:3", 2), None);
    }
}
