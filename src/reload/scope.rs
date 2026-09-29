//! what a reloaded config changes, so only that part is rebuilt

use std::fmt;

use crate::audio::dynamics::DynamicsConfig;
use crate::audio::spectrum::SpectrumConfig;
use crate::config::{Config, Theme};

/// the parts of the running overlay a new config touches
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Scope {
    pub analysis: bool,
    pub motion: bool,
    pub activity: bool,
    pub surfaces: bool,
}

impl Scope {
    pub fn between(
        old: &Config,
        old_theme: Option<&Theme>,
        new: &Config,
        new_theme: Option<&Theme>,
    ) -> Self {
        let framerate = old.visualizer.framerate != new.visualizer.framerate;
        Self {
            analysis: framerate
                || SpectrumConfig::from_config(old) != SpectrumConfig::from_config(new),
            motion: framerate
                || DynamicsConfig::from_config(old) != DynamicsConfig::from_config(new),
            activity: old.activity != new.activity,
            surfaces: old.overlay != new.overlay
                || old.visualizer != new.visualizer
                || old_theme != new_theme,
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = [
            (self.analysis, "analysis"),
            (self.motion, "motion"),
            (self.activity, "activity"),
            (self.surfaces, "surfaces"),
        ];
        let mut first = true;
        for (_, name) in parts.iter().filter(|(changed, _)| *changed) {
            if !first {
                f.write_str(", ")?;
            }
            f.write_str(name)?;
            first = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Scope;
    use crate::config::{Config, Rgba, Theme};

    fn scope(edit: impl FnOnce(&mut Config)) -> Scope {
        let old = Config::default();
        let mut new = old.clone();
        edit(&mut new);
        Scope::between(&old, None, &new, None)
    }

    #[test]
    fn nothing_changed_is_empty() {
        assert!(scope(|_| {}).is_empty());
        // the image overlay is not drawn yet
        assert!(scope(|c| c.image_overlay.enabled = !c.image_overlay.enabled).is_empty());
    }

    #[test]
    fn each_kind_of_key_reaches_its_part() {
        let colors = scope(|c| c.visualizer.color_rgba = Rgba::new(0.1, 0.2, 0.3, 1.0));
        assert_eq!(colors.to_string(), "surfaces");
        assert_eq!(scope(|c| c.overlay.margin_left = 3).to_string(), "surfaces");
        assert_eq!(scope(|c| c.audio.smoothing = 0.2).to_string(), "motion");
        assert_eq!(
            scope(|c| c.audio.low_cutoff_hz = 80.0).to_string(),
            "analysis"
        );
        assert_eq!(
            scope(|c| c.activity.threshold = 0.2).to_string(),
            "activity"
        );
        assert_eq!(
            scope(|c| c.visualizer.bars = 12).to_string(),
            "analysis, surfaces"
        );
        assert_eq!(
            scope(|c| c.visualizer.framerate = 144).to_string(),
            "analysis, motion, surfaces"
        );
    }

    #[test]
    fn a_new_theme_repaints_the_surfaces() {
        let config = Config::default();
        let theme = Theme {
            name: "test".to_owned(),
            colors: [Rgba::new(1.0, 0.0, 0.0, 1.0); 6],
        };
        let found = Scope::between(&config, None, &config, Some(&theme));
        assert_eq!(found.to_string(), "surfaces");
    }
}
