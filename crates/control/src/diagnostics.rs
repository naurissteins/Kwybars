use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use kwybars_common::config;
use kwybars_common::theme;

use crate::error::ControlError;

pub fn validate_config(path: &Path) -> Result<String, ControlError> {
    let explicit_path = path != config::default_config_path();
    if explicit_path && !path.exists() {
        return Err(ControlError::InvalidTarget(format!(
            "config does not exist: {}",
            path.display()
        )));
    }

    let resolved_config_path = resolve_runtime_config_path(path);
    let config_load_path = resolved_config_path.as_deref().unwrap_or(path);
    let mut loaded = config::load_or_default(config_load_path).map_err(ControlError::usage_like)?;
    let colors_path = config::default_colors_path(config_load_path);
    let colors_exists = colors_path.exists();

    if colors_exists {
        let overrides =
            config::load_color_overrides(&colors_path).map_err(ControlError::usage_like)?;
        config::apply_color_overrides(&mut loaded, overrides);
    }

    let mut theme_summary = "theme: none".to_owned();
    if let Some(theme_name) = loaded.visualizer.theme.as_deref() {
        let trimmed = theme_name.trim();
        if !trimmed.is_empty() {
            let theme_path = theme::resolve_theme_path(config_load_path, trimmed);
            let palette =
                theme::load_theme_palette(&theme_path, trimmed, loaded.visualizer.theme_opacity)
                    .map_err(ControlError::usage_like)?;
            theme_summary = format!("theme: {} ({})", palette.name, theme_path.display());
        }
    }

    let config_summary = if path.exists() {
        format!("config: {} (ok)", path.display())
    } else {
        format!(
            "config: {} (not found, built-in defaults valid)",
            path.display()
        )
    };
    let colors_summary = if colors_exists {
        format!("colors: {} (ok)", colors_path.display())
    } else {
        format!("colors: {} (not found)", colors_path.display())
    };

    Ok(format!(
        "config validation passed\n{config_summary}\n{colors_summary}\n{theme_summary}"
    ))
}

pub fn doctor(path: &Path) -> Result<String, ControlError> {
    let explicit_path = path != config::default_config_path();
    let resolved_config_path = resolve_runtime_config_path(path);
    let config_load_path = resolved_config_path.as_deref().unwrap_or(path);
    let colors_path = config::default_colors_path(config_load_path);
    let mut issues = Vec::new();
    let mut lines = vec!["kwybars doctor".to_owned()];

    let session_type = env::var("XDG_SESSION_TYPE").ok();
    let wayland_display = env::var("WAYLAND_DISPLAY").ok();
    let session_line = match (session_type.as_deref(), wayland_display.as_deref()) {
        (Some(session), Some(display)) => {
            format!("session: {session} (WAYLAND_DISPLAY={display})")
        }
        (Some(session), None) => format!("session: {session} (WAYLAND_DISPLAY not set)"),
        (None, Some(display)) => format!("session: unknown (WAYLAND_DISPLAY={display})"),
        (None, None) => "session: unknown (WAYLAND_DISPLAY not set)".to_owned(),
    };
    lines.push(session_line);
    if session_type.as_deref() != Some("wayland") && wayland_display.is_none() {
        issues.push("Wayland session not detected".to_owned());
    }

    let cava_path = find_in_path("cava");
    lines.push(match cava_path {
        Some(ref found) => format!("cava: found ({})", found.display()),
        None => "cava: not found in PATH".to_owned(),
    });

    let config_exists = path.exists();
    if explicit_path && !config_exists {
        issues.push(format!("Config does not exist: {}", path.display()));
        lines.push(format!("config: missing ({})", path.display()));
    } else if config_exists {
        lines.push(format!("config: found ({})", path.display()));
    } else {
        lines.push(format!(
            "config: not found ({}), built-in defaults will be used",
            path.display()
        ));
    }

    if let Some(resolved) = resolved_config_path.as_ref()
        && resolved != path
    {
        lines.push(format!("resolved config path: {}", resolved.display()));
    }

    let loaded = match config::load_or_default(config_load_path) {
        Ok(value) => {
            lines.push("config parse: ok".to_owned());
            Some(value)
        }
        Err(err) => {
            issues.push(format!("Config parse failed: {err}"));
            lines.push(format!("config parse: error ({err})"));
            None
        }
    };

    let colors_exists = colors_path.exists();
    if colors_exists {
        match config::load_color_overrides(&colors_path) {
            Ok(_) => lines.push(format!("colors: ok ({})", colors_path.display())),
            Err(err) => {
                issues.push(format!("Colors override parse failed: {err}"));
                lines.push(format!("colors: error ({err})"));
            }
        }
    } else {
        lines.push(format!("colors: not found ({})", colors_path.display()));
    }

    if let Some(config) = loaded.as_ref() {
        lines.push(format!("overlay.position: {}", config.overlay.position));
        lines.push(format!("overlay.layer: {}", config.overlay.layer));
        lines.push(format!(
            "overlay.monitor_mode: {}",
            config.overlay.monitor_mode
        ));
        lines.push(format!(
            "overlay.horizontal_alignment: {}",
            config.overlay.horizontal_alignment
        ));
        lines.push(format!(
            "overlay.vertical_alignment: {}",
            config.overlay.vertical_alignment
        ));
        lines.push(format!("visualizer.backend: {}", config.visualizer.backend));
        lines.push(format!("visualizer.layout: {}", config.visualizer.layout));
        lines.push(format!(
            "visualizer.line_mode: {}",
            config.visualizer.line_mode
        ));
        lines.push(format!(
            "visualizer.mirror_orientation: {}",
            config.visualizer.mirror_orientation
        ));
        lines.push(format!(
            "visualizer.mirror_gap: {}",
            config.visualizer.mirror_gap
        ));
        lines.push(format!(
            "visualizer.wave_stroke_width: {}",
            config.visualizer.wave_stroke_width
        ));
        lines.push(format!(
            "visualizer.wave_fill: {}",
            config.visualizer.wave_fill
        ));
        lines.push(format!(
            "visualizer.wave_glow: {}",
            config.visualizer.wave_glow
        ));
        lines.push(format!(
            "visualizer.wave_smoothing: {}",
            config.visualizer.wave_smoothing
        ));
        lines.push(format!(
            "visualizer.wave_motion_smoothing: {}",
            config.visualizer.wave_motion_smoothing
        ));
        lines.push(format!(
            "visualizer.wave_amplitude: {}",
            config.visualizer.wave_amplitude
        ));
        lines.push(format!(
            "visualizer.color_mode: {}",
            config.visualizer.color_mode
        ));
        lines.push(format!(
            "visualizer.frame_mirror_mode: {}",
            config.visualizer.frame_mirror_mode
        ));
        if matches!(
            config.visualizer.backend,
            config::VisualizerBackend::Cava | config::VisualizerBackend::Auto
        ) && cava_path.is_none()
        {
            issues.push("Backend requires `cava` in PATH".to_owned());
        }

        if let Some(theme_name) = config.visualizer.theme.as_deref() {
            let trimmed = theme_name.trim();
            if !trimmed.is_empty() {
                let theme_path = theme::resolve_theme_path(config_load_path, trimmed);
                match theme::load_theme_palette(
                    &theme_path,
                    trimmed,
                    config.visualizer.theme_opacity,
                ) {
                    Ok(palette) => lines.push(format!(
                        "theme: ok ({} -> {})",
                        palette.name,
                        theme_path.display()
                    )),
                    Err(err) => {
                        issues.push(format!("Theme load failed: {err}"));
                        lines.push(format!("theme: error ({err})"));
                    }
                }
            } else {
                lines.push("theme: none".to_owned());
            }
        } else {
            lines.push("theme: none".to_owned());
        }
    }

    if issues.is_empty() {
        lines.push("summary: ok".to_owned());
        Ok(lines.join("\n"))
    } else {
        lines.push(format!("summary: {} issue(s) found", issues.len()));
        Err(ControlError::Report(lines.join("\n")))
    }
}

pub fn list_themes(path: &Path) -> String {
    let resolved_config_path = resolve_runtime_config_path(path);
    let config_load_path = resolved_config_path.as_deref().unwrap_or(path);
    let themes = theme::list_available_themes(config_load_path);

    if themes.is_empty() {
        return "no themes found".to_owned();
    }

    let mut lines = vec!["available themes".to_owned()];
    for theme in themes {
        lines.push(format!(
            "- {} ({}, {})",
            theme.name,
            theme.source.label(),
            theme.path.display()
        ));
    }
    lines.join("\n")
}

fn resolve_runtime_config_path(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

fn find_in_path(binary: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.is_file())
}
