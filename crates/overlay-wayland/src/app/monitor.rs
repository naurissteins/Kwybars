use std::collections::BTreeSet;

use kwybars_common::config::{OverlayConfig, OverlayMonitorMode};
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::client::protocol::wl_output;

pub enum OutputSelection {
    Ready(Vec<wl_output::WlOutput>),
    PendingNames,
}

pub fn select_outputs(output_state: &OutputState, overlay: &OverlayConfig) -> OutputSelection {
    let outputs: Vec<_> = output_state.outputs().collect();
    if outputs.is_empty() {
        return OutputSelection::Ready(Vec::new());
    }

    match overlay.monitor_mode {
        OverlayMonitorMode::Primary => OutputSelection::Ready(vec![outputs[0].clone()]),
        OverlayMonitorMode::All => OutputSelection::Ready(outputs),
        OverlayMonitorMode::List => {
            resolve_requested_outputs(output_state, outputs, &overlay.monitors)
        }
    }
}

fn resolve_requested_outputs(
    output_state: &OutputState,
    outputs: Vec<wl_output::WlOutput>,
    requested: &[String],
) -> OutputSelection {
    if requested.is_empty() {
        return OutputSelection::Ready(vec![outputs[0].clone()]);
    }

    let mut used_indices = BTreeSet::new();
    let mut selected = Vec::new();
    let mut pending_named_resolution = false;

    for requested_name in requested {
        let requested_name = requested_name.trim();
        if requested_name.is_empty() {
            continue;
        }

        if let Some(index) = parse_monitor_index(requested_name, outputs.len()) {
            if used_indices.insert(index) {
                selected.push(outputs[index].clone());
            }
            continue;
        }

        let mut missing_name_metadata = false;
        let Some(index) = outputs.iter().enumerate().find_map(|(index, output)| {
            let Some(info) = output_state.info(output) else {
                missing_name_metadata = true;
                return None;
            };
            match info.name.as_deref() {
                Some(name) if name == requested_name => Some(index),
                Some(_) => None,
                None => {
                    missing_name_metadata = true;
                    None
                }
            }
        }) else {
            if missing_name_metadata {
                pending_named_resolution = true;
            }
            continue;
        };

        if used_indices.insert(index) {
            selected.push(outputs[index].clone());
        }
    }

    if pending_named_resolution {
        OutputSelection::PendingNames
    } else if selected.is_empty() {
        OutputSelection::Ready(vec![outputs[0].clone()])
    } else {
        OutputSelection::Ready(selected)
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

#[cfg(test)]
mod tests {
    use super::parse_monitor_index;

    #[test]
    fn parses_one_based_monitor_indices() {
        assert_eq!(parse_monitor_index("1", 3), Some(0));
        assert_eq!(parse_monitor_index("index:2", 3), Some(1));
        assert_eq!(parse_monitor_index("0", 3), None);
        assert_eq!(parse_monitor_index("4", 3), None);
    }
}
