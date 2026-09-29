use crate::config::{MonitorMode, OverlayConfig};

/// one surface to create
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub output: usize,
    pub entry: Option<usize>,
}

/// outputs to cover and anything worth a warning
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub targets: Vec<Target>,
    pub warnings: Vec<String>,
}

/// picks outputs by name, names holds each output's name in advertisement
/// order, and the first output is the primary one
pub fn select(overlay: &OverlayConfig, names: &[Option<&str>]) -> Selection {
    let mut selection = Selection::default();
    if names.is_empty() {
        return selection;
    }
    if !overlay.outputs.is_empty() {
        for (entry, output) in overlay.outputs.iter().enumerate() {
            if !output.enabled {
                continue;
            }
            match find(&output.monitor, names) {
                Some(index) => selection.add(index, Some(entry), &output.monitor),
                None => selection.warnings.push(format!(
                    "overlay.outputs: no connected output named {:?}",
                    output.monitor
                )),
            }
        }
        return selection;
    }
    match overlay.monitor_mode {
        MonitorMode::Primary => selection.add(0, None, "primary"),
        MonitorMode::All => {
            for index in 0..names.len() {
                selection.add(index, None, "");
            }
        }
        MonitorMode::List => {
            for name in &overlay.monitors {
                if let Some(index) = find(name, names) {
                    selection.add(index, None, name);
                }
            }
            if selection.targets.is_empty() {
                selection.add(0, None, "primary");
            }
        }
    }
    selection
}

impl Selection {
    fn add(&mut self, output: usize, entry: Option<usize>, requested: &str) {
        if self.targets.iter().any(|target| target.output == output) {
            if entry.is_some() {
                self.warnings.push(format!(
                    "overlay.outputs: {requested:?} names an output that already has an overlay, ignored"
                ));
            }
            return;
        }
        self.targets.push(Target { output, entry });
    }
}

fn find(requested: &str, names: &[Option<&str>]) -> Option<usize> {
    let requested = requested.trim();
    if requested.is_empty() {
        return None;
    }
    if requested == "primary" {
        return Some(0);
    }
    let number = requested.strip_prefix("index:").unwrap_or(requested);
    if let Ok(one_based) = number.parse::<usize>() {
        return one_based
            .checked_sub(1)
            .filter(|index| *index < names.len());
    }
    names.iter().position(|name| *name == Some(requested))
}

#[cfg(test)]
mod tests {
    use super::{Selection, Target, select};
    use crate::config::{MonitorMode, OutputConfig, OverlayConfig};

    const NAMES: [Option<&str>; 3] = [Some("DP-4"), Some("DP-1"), Some("DP-2")];

    fn outputs(indices: &[usize]) -> Vec<Target> {
        indices
            .iter()
            .map(|&output| Target {
                output,
                entry: None,
            })
            .collect()
    }

    fn list(monitors: &[&str]) -> OverlayConfig {
        OverlayConfig {
            monitor_mode: MonitorMode::List,
            monitors: monitors.iter().map(|name| (*name).to_owned()).collect(),
            ..OverlayConfig::default()
        }
    }

    fn entry(monitor: &str, enabled: bool) -> OutputConfig {
        OutputConfig {
            monitor: monitor.to_owned(),
            enabled,
            ..OutputConfig::default()
        }
    }

    #[test]
    fn primary_is_the_first_advertised_output() {
        let selection = select(&OverlayConfig::default(), &NAMES);
        assert_eq!(selection.targets, outputs(&[0]));
    }

    #[test]
    fn all_covers_every_output() {
        let overlay = OverlayConfig {
            monitor_mode: MonitorMode::All,
            ..OverlayConfig::default()
        };
        assert_eq!(select(&overlay, &NAMES).targets, outputs(&[0, 1, 2]));
    }

    #[test]
    fn list_matches_names_and_indices_without_duplicates() {
        let overlay = list(&["DP-2", " index:1 ", "3", "HDMI-A-1", "primary"]);
        assert_eq!(
            select(&overlay, &NAMES),
            Selection {
                targets: outputs(&[2, 0]),
                warnings: Vec::new(),
            }
        );
    }

    #[test]
    fn list_without_a_match_falls_back_to_the_primary() {
        assert_eq!(
            select(&list(&["HDMI-A-1", "0", "9"]), &NAMES).targets,
            outputs(&[0])
        );
        assert_eq!(select(&list(&[]), &NAMES).targets, outputs(&[0]));
    }

    #[test]
    fn output_entries_replace_the_mode() {
        let overlay = OverlayConfig {
            monitor_mode: MonitorMode::All,
            outputs: vec![
                entry("DP-2", true),
                entry("DP-1", false),
                entry("HDMI-A-1", true),
                entry("index:3", true),
                entry("DP-4", true),
            ],
            ..OverlayConfig::default()
        };
        let selection = select(&overlay, &NAMES);
        assert_eq!(
            selection.targets,
            vec![
                Target {
                    output: 2,
                    entry: Some(0),
                },
                Target {
                    output: 0,
                    entry: Some(4),
                },
            ]
        );
        assert_eq!(selection.warnings.len(), 2, "{:?}", selection.warnings);
    }

    #[test]
    fn nameless_outputs_are_reachable_by_index_only() {
        let names = [None, Some("DP-1")];
        assert_eq!(
            select(&list(&["DP-1", "1"]), &names).targets,
            outputs(&[1, 0])
        );
    }

    #[test]
    fn no_outputs_no_targets() {
        assert_eq!(select(&OverlayConfig::default(), &[]), Selection::default());
    }
}
