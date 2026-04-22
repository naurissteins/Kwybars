use std::path::{Path, PathBuf};

use kwybars_common::config::{self, AppConfig};
use kwybars_common::theme::{self, ThemePalette};
use tracing::{info, warn};

use super::AppError;

pub struct RuntimeConfig {
    pub app_config: AppConfig,
    pub config_load_path: PathBuf,
    pub colors_path: PathBuf,
    pub theme_palette: Option<ThemePalette>,
    pub theme_path: Option<PathBuf>,
}

pub fn load(config_path: &Path) -> Result<RuntimeConfig, AppError> {
    let config_load_path =
        std::fs::canonicalize(config_path).unwrap_or_else(|_| config_path.into());
    let mut app_config = config::load_or_default(&config_load_path).map_err(AppError::Config)?;
    let colors_path = config::default_colors_path(&config_load_path);
    match config::load_color_overrides(&colors_path) {
        Ok(overrides) => config::apply_color_overrides(&mut app_config, overrides),
        Err(err) => {
            warn!("kwybars-overlay-next: colors override load failed, using config colors: {err}");
        }
    }

    let (theme_palette, theme_path) = load_theme_for_config(&app_config, &config_load_path);
    Ok(RuntimeConfig {
        app_config,
        config_load_path,
        colors_path,
        theme_palette,
        theme_path,
    })
}

fn load_theme_for_config(
    config: &AppConfig,
    config_path: &Path,
) -> (Option<ThemePalette>, Option<PathBuf>) {
    let Some(theme_name) = config.visualizer.theme.as_deref() else {
        return (None, None);
    };
    let trimmed_name = theme_name.trim();
    if trimmed_name.is_empty() {
        return (None, None);
    }

    let theme_path = theme::resolve_theme_path(config_path, trimmed_name);
    match theme::load_theme_palette(&theme_path, trimmed_name, config.visualizer.theme_opacity) {
        Ok(palette) => {
            info!("theme loaded: {} ({})", palette.name, theme_path.display());
            (Some(palette), Some(theme_path))
        }
        Err(err) => {
            warn!("kwybars-overlay-next: theme load failed, using configured colors: {err}");
            (None, Some(theme_path))
        }
    }
}
