//! toml text to a checked [`Config`]

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_ignored::Path;

use super::Config;
use super::activity::{self, ActivityTable, DaemonTable};
use super::audio::AudioConfig;
use super::compat;
use super::image::ImageOverlayConfig;
use super::overlay::{self, OutputConfig, OverlayTable};
use super::visualizer::{VisualizerConfig, VisualizerOverrides};

/// a parsed config plus warnings about keys that were ignored or fixed
#[derive(Debug)]
pub struct Parsed {
    pub config: Config,
    pub warnings: Vec<String>,
}

/// config text that cannot be used
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error("{0}")]
    Invalid(String),
}

/// the file layout; root `theme` keys are a shorthand for `[visualizer]`
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ConfigFile {
    theme: Option<String>,
    theme_opacity: Option<f32>,
    overlay: OverlayTable,
    output: BTreeMap<String, OutputConfig>,
    visualizer: VisualizerOverrides,
    image_overlay: ImageOverlayConfig,
    activity: ActivityTable,
    daemon: DaemonTable,
    audio: AudioConfig,
}

/// parses config text; unknown keys become warnings, bad types become errors
pub fn parse(raw: &str) -> Result<Parsed, ParseError> {
    let mut warnings = Vec::new();
    let mut removed = Vec::new();
    let deserializer = toml::de::Deserializer::parse(raw)?;
    let file: ConfigFile = serde_ignored::deserialize(deserializer, |path| {
        let key = key_path(&path);
        match compat::removed_reason(&key) {
            Some(reason) => removed.push((key, reason)),
            None => warnings.push(match compat::hint(&key) {
                Some(hint) => format!("{key}: unknown key, ignored ({hint})"),
                None => format!("{key}: unknown key, ignored"),
            }),
        }
    })?;
    warnings.extend(compat::removed_warnings(&removed));
    let config = build(file, &mut warnings)?;
    Ok(Parsed { config, warnings })
}

fn build(file: ConfigFile, warnings: &mut Vec<String>) -> Result<Config, ParseError> {
    let ConfigFile {
        theme,
        theme_opacity,
        overlay,
        output,
        visualizer: mut overrides,
        mut image_overlay,
        activity,
        daemon,
        mut audio,
    } = file;

    // [visualizer] wins over the root shorthand
    if overrides.theme.is_none() {
        overrides.theme = theme;
    }
    if overrides.theme_opacity.is_none() {
        overrides.theme_opacity = theme_opacity;
    }
    overrides.normalize("visualizer", warnings);
    let mut visualizer = VisualizerConfig::default();
    overrides.apply_to(&mut visualizer);

    let overlay = overlay::resolve(overlay, output, warnings).map_err(ParseError::Invalid)?;

    image_overlay.normalize(warnings);
    let activity = activity::resolve(activity, daemon, warnings);
    audio.normalize(warnings);

    Ok(Config {
        overlay,
        visualizer,
        image_overlay,
        activity,
        audio,
    })
}

/// formats an ignored key as `overlay.outputs[0].visualizer.foo`
fn key_path(path: &Path<'_>) -> String {
    match path {
        Path::Root => String::new(),
        Path::Seq { parent, index } => format!("{}[{index}]", key_path(parent)),
        Path::Map { parent, key } => match key_path(parent) {
            prefix if prefix.is_empty() => key.clone(),
            prefix => format!("{prefix}.{key}"),
        },
        Path::Some { parent }
        | Path::NewtypeStruct { parent }
        | Path::NewtypeVariant { parent } => key_path(parent),
    }
}
