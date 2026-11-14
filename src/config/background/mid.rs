use serde::Deserialize;

use crate::config::bounds::Bounds;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct MidConfig {
    pub enabled: bool,
    pub low_cutoff_hz: f32,
    pub high_cutoff_hz: f32,
    pub sensitivity: f32,
    pub threshold: f32,
    pub zoom: f32,
    pub shake: f32,
    pub attack_ms: u32,
    pub release_ms: u32,
}

impl Default for MidConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            low_cutoff_hz: 250.0,
            high_cutoff_hz: 4000.0,
            sensitivity: 20.0,
            threshold: 0.001,
            zoom: 0.0,
            shake: 0.003,
            attack_ms: 8,
            release_ms: 110,
        }
    }
}

impl MidConfig {
    pub(super) fn normalize(&mut self, warnings: &mut Vec<String>) {
        let mut values = MidOverrides {
            enabled: Some(self.enabled),
            low_cutoff_hz: Some(self.low_cutoff_hz),
            high_cutoff_hz: Some(self.high_cutoff_hz),
            sensitivity: Some(self.sensitivity),
            threshold: Some(self.threshold),
            zoom: Some(self.zoom),
            shake: Some(self.shake),
            attack_ms: Some(self.attack_ms),
            release_ms: Some(self.release_ms),
        };
        values.normalize("background.mid", false, warnings);
        *self = values.over(&Self::default());
        if self.high_cutoff_hz < self.low_cutoff_hz * 2.0 {
            self.high_cutoff_hz = self.low_cutoff_hz * 2.0;
            warnings.push(format!(
                "background.mid.high_cutoff_hz: must be at least twice low_cutoff_hz, using {}",
                self.high_cutoff_hz
            ));
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct MidOverrides {
    pub enabled: Option<bool>,
    pub low_cutoff_hz: Option<f32>,
    pub high_cutoff_hz: Option<f32>,
    pub sensitivity: Option<f32>,
    pub threshold: Option<f32>,
    pub zoom: Option<f32>,
    pub shake: Option<f32>,
    pub attack_ms: Option<u32>,
    pub release_ms: Option<u32>,
}

impl MidOverrides {
    pub fn over(&self, base: &MidConfig) -> MidConfig {
        MidConfig {
            enabled: self.enabled.unwrap_or(base.enabled),
            low_cutoff_hz: self.low_cutoff_hz.unwrap_or(base.low_cutoff_hz),
            high_cutoff_hz: self.high_cutoff_hz.unwrap_or(base.high_cutoff_hz),
            sensitivity: self.sensitivity.unwrap_or(base.sensitivity),
            threshold: self.threshold.unwrap_or(base.threshold),
            zoom: self.zoom.unwrap_or(base.zoom),
            shake: self.shake.unwrap_or(base.shake),
            attack_ms: self.attack_ms.unwrap_or(base.attack_ms),
            release_ms: self.release_ms.unwrap_or(base.release_ms),
        }
    }

    pub(super) fn normalize(&mut self, table: &str, output: bool, warnings: &mut Vec<String>) {
        let mut bounds = Bounds::new(table, warnings);
        bounds.within("sensitivity", &mut self.sensitivity, 0.0, 1000.0);
        bounds.within("threshold", &mut self.threshold, 0.0, 1.0);
        bounds.within("zoom", &mut self.zoom, 0.0, 0.25);
        bounds.within("shake", &mut self.shake, 0.0, 0.02);
        bounds.between("attack_ms", &mut self.attack_ms, 1, 5000);
        bounds.between("release_ms", &mut self.release_ms, 1, 5000);
        if output {
            for (key, value) in [
                ("low_cutoff_hz", &mut self.low_cutoff_hz),
                ("high_cutoff_hz", &mut self.high_cutoff_hz),
            ] {
                bounds.unsupported(key, value, "set in [background.mid] for all outputs");
            }
        } else {
            bounds.within("low_cutoff_hz", &mut self.low_cutoff_hz, 100.0, 10_000.0);
            bounds.within("high_cutoff_hz", &mut self.high_cutoff_hz, 200.0, 20_000.0);
        }
    }
}
