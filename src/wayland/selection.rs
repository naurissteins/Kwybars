use crate::config::{OverlayConfig, ShowOn};

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
    match &overlay.show_on {
        ShowOn::Sections => selection.sections(overlay, names),
        ShowOn::Primary => selection.cover(overlay, names, 0),
        ShowOn::All => {
            for output in 0..names.len() {
                selection.cover(overlay, names, output);
            }
        }
        ShowOn::Named(wanted) => {
            let mut connected = wanted
                .iter()
                .filter_map(|name| find(name, names))
                .peekable();
            if connected.peek().is_none() {
                selection.cover(overlay, names, 0);
            }
            for output in connected {
                selection.cover(overlay, names, output);
            }
        }
    }
    selection
}

impl Selection {
    /// every enabled section's output, in section order
    fn sections(&mut self, overlay: &OverlayConfig, names: &[Option<&str>]) {
        for (entry, section) in overlay.outputs.iter().enumerate() {
            if !section.enabled {
                continue;
            }
            let monitor = &section.monitor;
            match find(monitor, names) {
                Some(output) if self.covers(output) => self.warnings.push(format!(
                    "[output.{monitor}] names an output that already has an overlay, ignored"
                )),
                Some(output) => self.targets.push(Target {
                    output,
                    entry: Some(entry),
                }),
                None => self.warnings.push(format!(
                    "[output.{monitor}]: no connected output has that name"
                )),
            }
        }
    }

    /// adds a chosen output with its section, unless that section disables it
    fn cover(&mut self, overlay: &OverlayConfig, names: &[Option<&str>], output: usize) {
        if self.covers(output) {
            return;
        }
        let entry = overlay
            .outputs
            .iter()
            .position(|section| find(&section.monitor, names) == Some(output));
        let disabled = entry
            .and_then(|entry| overlay.outputs.get(entry))
            .is_some_and(|section| !section.enabled);
        if !disabled {
            self.targets.push(Target { output, entry });
        }
    }

    fn covers(&self, output: usize) -> bool {
        self.targets.iter().any(|target| target.output == output)
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
    use crate::config::{OutputConfig, OverlayConfig, ShowOn};

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
            show_on: ShowOn::Named(monitors.iter().map(|name| (*name).to_owned()).collect()),
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
            show_on: ShowOn::All,
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
    fn sections_alone_choose_their_outputs() {
        let overlay = OverlayConfig {
            show_on: ShowOn::Sections,
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
    fn show_on_chooses_and_sections_only_adjust_or_disable() {
        let sections = vec![
            entry("DP-1", true),
            entry("DP-2", false),
            entry("HDMI-A-1", true),
        ];
        let all = OverlayConfig {
            show_on: ShowOn::All,
            outputs: sections.clone(),
            ..OverlayConfig::default()
        };
        assert_eq!(
            select(&all, &NAMES),
            Selection {
                targets: vec![
                    Target {
                        output: 0,
                        entry: None,
                    },
                    Target {
                        output: 1,
                        entry: Some(0),
                    },
                ],
                warnings: Vec::new(),
            }
        );

        // a section for a monitor that is not chosen changes nothing
        let one = OverlayConfig {
            show_on: ShowOn::Named(vec!["DP-4".to_owned()]),
            outputs: sections.clone(),
            ..OverlayConfig::default()
        };
        assert_eq!(select(&one, &NAMES).targets, outputs(&[0]));

        // the only chosen monitor is disabled by its section
        let off = OverlayConfig {
            show_on: ShowOn::Named(vec!["DP-2".to_owned()]),
            outputs: sections,
            ..OverlayConfig::default()
        };
        assert_eq!(select(&off, &NAMES), Selection::default());
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
