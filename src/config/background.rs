//! `[background]`: persistent wallpaper and its independent bass response

use serde::Deserialize;

use super::bounds::Bounds;
use super::{ImageFit, ImageOverlayConfig};

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct BackgroundConfig {
    pub enabled: bool,
    pub path: Option<String>,
    pub low_cutoff_hz: f32,
    pub high_cutoff_hz: f32,
    pub sensitivity: f32,
    pub threshold: f32,
    pub zoom: f32,
    pub shake: f32,
    pub attack_ms: u32,
    pub release_ms: u32,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: None,
            low_cutoff_hz: 30.0,
            high_cutoff_hz: 150.0,
            sensitivity: 6.7,
            threshold: 0.012,
            zoom: 0.04,
            shake: 0.0,
            attack_ms: 35,
            release_ms: 220,
        }
    }
}

impl BackgroundConfig {
    pub fn image(&self) -> ImageOverlayConfig {
        ImageOverlayConfig {
            enabled: self.enabled,
            path: self.path.clone(),
            fit: ImageFit::Cover,
            ..ImageOverlayConfig::default()
        }
    }

    pub(crate) fn normalize(&mut self, warnings: &mut Vec<String>) {
        let mut overrides = BackgroundOverrides {
            enabled: Some(self.enabled),
            path: self.path.clone(),
            low_cutoff_hz: Some(self.low_cutoff_hz),
            high_cutoff_hz: Some(self.high_cutoff_hz),
            sensitivity: Some(self.sensitivity),
            threshold: Some(self.threshold),
            zoom: Some(self.zoom),
            shake: Some(self.shake),
            attack_ms: Some(self.attack_ms),
            release_ms: Some(self.release_ms),
        };
        overrides.normalize("background", false, warnings);
        *self = overrides.over(&Self::default());
        if self.high_cutoff_hz < self.low_cutoff_hz * 2.0 {
            self.high_cutoff_hz = self.low_cutoff_hz * 2.0;
            warnings.push(format!(
                "background.high_cutoff_hz: must be at least twice low_cutoff_hz, using {}",
                self.high_cutoff_hz
            ));
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct BackgroundOverrides {
    pub enabled: Option<bool>,
    pub path: Option<String>,
    pub low_cutoff_hz: Option<f32>,
    pub high_cutoff_hz: Option<f32>,
    pub sensitivity: Option<f32>,
    pub threshold: Option<f32>,
    pub zoom: Option<f32>,
    pub shake: Option<f32>,
    pub attack_ms: Option<u32>,
    pub release_ms: Option<u32>,
}

impl BackgroundOverrides {
    pub fn over(&self, base: &BackgroundConfig) -> BackgroundConfig {
        let mut config = base.clone();
        macro_rules! apply { ($($field:ident),*) => { $(if let Some(value) = self.$field { config.$field = value; })* }; }
        apply!(
            enabled,
            low_cutoff_hz,
            high_cutoff_hz,
            sensitivity,
            threshold,
            zoom,
            shake,
            attack_ms,
            release_ms
        );
        config.path = self.path.clone().or_else(|| base.path.clone());
        config
    }

    pub(crate) fn normalize(&mut self, table: &str, output: bool, warnings: &mut Vec<String>) {
        let mut bounds = Bounds::new(table, warnings);
        bounds.within("sensitivity", &mut self.sensitivity, 0.0, 1000.0);
        bounds.within("threshold", &mut self.threshold, 0.0, 1.0);
        bounds.within("zoom", &mut self.zoom, 0.0, 0.25);
        bounds.within("shake", &mut self.shake, 0.0, 0.02);
        bounds.between("attack_ms", &mut self.attack_ms, 1, 5000);
        bounds.between("release_ms", &mut self.release_ms, 1, 5000);
        if output {
            bounds.unsupported(
                "low_cutoff_hz",
                &mut self.low_cutoff_hz,
                "set in [background] for all outputs",
            );
            bounds.unsupported(
                "high_cutoff_hz",
                &mut self.high_cutoff_hz,
                "set in [background] for all outputs",
            );
        } else {
            bounds.within("low_cutoff_hz", &mut self.low_cutoff_hz, 20.0, 1000.0);
            bounds.within("high_cutoff_hz", &mut self.high_cutoff_hz, 40.0, 2000.0);
        }
        self.path = self
            .path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_owned);
    }
}
