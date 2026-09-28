//! [activity]: when audio counts as playing, with the legacy [daemon] alias

use serde::Deserialize;

use super::bounds::Bounds;

/// the resolved [activity] settings
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityConfig {
    /// peak level in `0.0..=1.0` above which audio counts as playing
    pub threshold: f32,
    /// how long audio must stay above the threshold before showing
    pub activate_delay_ms: u64,
    /// how long audio must stay below the threshold before hiding
    pub deactivate_delay_ms: u64,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        Self {
            threshold: 0.035,
            activate_delay_ms: 180,
            deactivate_delay_ms: 2200,
        }
    }
}

/// the [activity] table as written
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct ActivityTable {
    threshold: Option<f32>,
    activate_delay_ms: Option<u64>,
    deactivate_delay_ms: Option<u64>,
}

/// activity keys of the legacy [daemon] table, its other keys are removed
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct DaemonTable {
    activity_threshold: Option<f32>,
    activate_delay_ms: Option<u64>,
    deactivate_delay_ms: Option<u64>,
}

/// merges [activity] over the deprecated [daemon] keys over the defaults
pub(super) fn resolve(
    activity: ActivityTable,
    daemon: DaemonTable,
    warnings: &mut Vec<String>,
) -> ActivityConfig {
    let mut alias = Alias::default();
    let mut threshold = alias.pick(
        activity.threshold,
        daemon.activity_threshold,
        "activity_threshold",
        "threshold",
    );
    let activate = alias.pick(
        activity.activate_delay_ms,
        daemon.activate_delay_ms,
        "activate_delay_ms",
        "activate_delay_ms",
    );
    let deactivate = alias.pick(
        activity.deactivate_delay_ms,
        daemon.deactivate_delay_ms,
        "deactivate_delay_ms",
        "deactivate_delay_ms",
    );
    alias.report(warnings);
    Bounds::new("activity", warnings).within("threshold", &mut threshold, 0.0, 1.0);

    let defaults = ActivityConfig::default();
    ActivityConfig {
        threshold: threshold.unwrap_or(defaults.threshold),
        activate_delay_ms: activate.unwrap_or(defaults.activate_delay_ms),
        deactivate_delay_ms: deactivate.unwrap_or(defaults.deactivate_delay_ms),
    }
}

/// tracks which legacy [daemon] keys were used or overridden
#[derive(Default)]
struct Alias {
    used: Vec<String>,
    shadowed: Vec<&'static str>,
}

impl Alias {
    /// [activity] value if set, else the legacy one
    fn pick<T>(
        &mut self,
        own: Option<T>,
        legacy: Option<T>,
        legacy_key: &'static str,
        new_key: &str,
    ) -> Option<T> {
        match (own, legacy) {
            (Some(value), Some(_)) => {
                self.shadowed.push(legacy_key);
                Some(value)
            }
            (None, Some(value)) => {
                self.used
                    .push(format!("daemon.{legacy_key} -> activity.{new_key}"));
                Some(value)
            }
            (own, None) => own,
        }
    }

    fn report(self, warnings: &mut Vec<String>) {
        if !self.used.is_empty() {
            warnings.push(format!(
                "[daemon] is deprecated, move these keys to [activity]: {}",
                self.used.join(", ")
            ));
        }
        for key in self.shadowed {
            warnings.push(format!("daemon.{key}: ignored, [activity] sets it"));
        }
    }
}
