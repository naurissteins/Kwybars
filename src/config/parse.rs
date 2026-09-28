//! toml text to a checked [`Config`]

use serde::Deserialize;
use serde_ignored::Path;

use super::Config;
use super::image::ImageOverlayConfig;
use super::overlay::OverlayConfig;
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
    overlay: OverlayConfig,
    visualizer: VisualizerOverrides,
    image_overlay: ImageOverlayConfig,
}

/// parses config text; unknown keys become warnings, bad types become errors
pub fn parse(raw: &str) -> Result<Parsed, ParseError> {
    let mut warnings = Vec::new();
    let deserializer = toml::de::Deserializer::parse(raw)?;
    let file: ConfigFile = serde_ignored::deserialize(deserializer, |path| {
        warnings.push(format!("{}: unknown key, ignored", key_path(&path)));
    })?;
    let config = build(file, &mut warnings)?;
    Ok(Parsed { config, warnings })
}

fn build(file: ConfigFile, warnings: &mut Vec<String>) -> Result<Config, ParseError> {
    let ConfigFile {
        theme,
        theme_opacity,
        mut overlay,
        visualizer: mut overrides,
        mut image_overlay,
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

    overlay.monitors = clean_names(&overlay.monitors);
    for (index, output) in overlay.outputs.iter_mut().enumerate() {
        let table = format!("overlay.outputs[{index}]");
        output.monitor = output.monitor.trim().to_owned();
        if output.monitor.is_empty() {
            return Err(ParseError::Invalid(format!("{table}: missing `monitor`")));
        }
        let table = format!("{table}.visualizer");
        output.visualizer.drop_global_keys(&table, warnings);
        output.visualizer.normalize(&table, warnings);
    }

    image_overlay.normalize(warnings);

    Ok(Config {
        overlay,
        visualizer,
        image_overlay,
    })
}

fn clean_names(names: &[String]) -> Vec<String> {
    names
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
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
