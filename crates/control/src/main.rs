use std::env;
use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use kwybars_common::config;
use kwybars_common::theme;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    SwitchConfig {
        target: PathBuf,
        active: Option<PathBuf>,
    },
    ValidateConfig {
        path: Option<PathBuf>,
    },
    Doctor {
        path: Option<PathBuf>,
    },
    ListThemes {
        path: Option<PathBuf>,
    },
    Help,
}

#[derive(Debug)]
enum ControlError {
    Usage(String),
    Io(std::io::Error),
    InvalidTarget(String),
    Report(String),
}

impl Display for ControlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(f, "{message}"),
            Self::Io(err) => write!(f, "{err}"),
            Self::InvalidTarget(message) => write!(f, "{message}"),
            Self::Report(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ControlError {}

impl From<std::io::Error> for ControlError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

fn main() {
    match run() {
        Ok(Some(message)) => println!("{message}"),
        Ok(None) => {}
        Err(ControlError::Usage(message)) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
        Err(ControlError::Report(message)) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("kwybarsctl: {err}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<Option<String>, ControlError> {
    match parse_args(std::env::args_os().skip(1))? {
        Command::Help => Ok(Some(usage())),
        Command::SwitchConfig { target, active } => {
            let active_path = active.unwrap_or_else(config::default_config_path);
            let target_path = validate_target(&target)?;
            let message = switch_config(&active_path, &target_path)?;
            Ok(Some(message))
        }
        Command::ValidateConfig { path } => {
            let path = path.unwrap_or_else(config::default_config_path);
            let message = validate_config(&path)?;
            Ok(Some(message))
        }
        Command::Doctor { path } => {
            let path = path.unwrap_or_else(config::default_config_path);
            let message = doctor(&path)?;
            Ok(Some(message))
        }
        Command::ListThemes { path } => {
            let path = path.unwrap_or_else(config::default_config_path);
            let message = list_themes(&path);
            Ok(Some(message))
        }
    }
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Command, ControlError> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Ok(Command::Help);
    };

    match command.to_string_lossy().as_ref() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "switch-config" => parse_switch_config(args),
        "validate-config" => parse_validate_config(args),
        "doctor" => parse_doctor(args),
        "list-themes" => parse_list_themes(args),
        other => Err(ControlError::Usage(format!(
            "unknown command: {other}\n\n{}",
            usage()
        ))),
    }
}

fn parse_switch_config(args: impl IntoIterator<Item = OsString>) -> Result<Command, ControlError> {
    let mut args = args.into_iter();
    let mut active = None;
    let mut target = None;

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "-h" | "--help" => return Ok(Command::Help),
            "-a" | "--active" => {
                let Some(value) = args.next() else {
                    return Err(ControlError::Usage(format!(
                        "missing value for --active\n\n{}",
                        usage()
                    )));
                };
                active = Some(PathBuf::from(value));
            }
            value if value.starts_with("--active=") => {
                let path = &value["--active=".len()..];
                if path.is_empty() {
                    return Err(ControlError::Usage(format!(
                        "missing value for --active\n\n{}",
                        usage()
                    )));
                }
                active = Some(PathBuf::from(path));
            }
            other => {
                if target.is_some() {
                    return Err(ControlError::Usage(format!(
                        "unexpected extra argument: {other}\n\n{}",
                        usage()
                    )));
                }
                target = Some(PathBuf::from(other));
            }
        }
    }

    let Some(target) = target else {
        return Err(ControlError::Usage(format!(
            "missing target config path\n\n{}",
            usage()
        )));
    };

    Ok(Command::SwitchConfig { target, active })
}

fn parse_validate_config(
    args: impl IntoIterator<Item = OsString>,
) -> Result<Command, ControlError> {
    let mut args = args.into_iter();
    let mut path = None;

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "-h" | "--help" => return Ok(Command::Help),
            "-c" | "--config" => {
                let Some(value) = args.next() else {
                    return Err(ControlError::Usage(format!(
                        "missing value for --config\n\n{}",
                        usage()
                    )));
                };
                path = Some(PathBuf::from(value));
            }
            value if value.starts_with("--config=") => {
                let path_value = &value["--config=".len()..];
                if path_value.is_empty() {
                    return Err(ControlError::Usage(format!(
                        "missing value for --config\n\n{}",
                        usage()
                    )));
                }
                path = Some(PathBuf::from(path_value));
            }
            other => {
                if path.is_some() {
                    return Err(ControlError::Usage(format!(
                        "unexpected extra argument: {other}\n\n{}",
                        usage()
                    )));
                }
                path = Some(PathBuf::from(other));
            }
        }
    }

    Ok(Command::ValidateConfig { path })
}

fn parse_doctor(args: impl IntoIterator<Item = OsString>) -> Result<Command, ControlError> {
    let mut args = args.into_iter();
    let mut path = None;

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "-h" | "--help" => return Ok(Command::Help),
            "-c" | "--config" => {
                let Some(value) = args.next() else {
                    return Err(ControlError::Usage(format!(
                        "missing value for --config\n\n{}",
                        usage()
                    )));
                };
                path = Some(PathBuf::from(value));
            }
            value if value.starts_with("--config=") => {
                let path_value = &value["--config=".len()..];
                if path_value.is_empty() {
                    return Err(ControlError::Usage(format!(
                        "missing value for --config\n\n{}",
                        usage()
                    )));
                }
                path = Some(PathBuf::from(path_value));
            }
            other => {
                if path.is_some() {
                    return Err(ControlError::Usage(format!(
                        "unexpected extra argument: {other}\n\n{}",
                        usage()
                    )));
                }
                path = Some(PathBuf::from(other));
            }
        }
    }

    Ok(Command::Doctor { path })
}

fn parse_list_themes(args: impl IntoIterator<Item = OsString>) -> Result<Command, ControlError> {
    let mut args = args.into_iter();
    let mut path = None;

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "-h" | "--help" => return Ok(Command::Help),
            "-c" | "--config" => {
                let Some(value) = args.next() else {
                    return Err(ControlError::Usage(format!(
                        "missing value for --config\n\n{}",
                        usage()
                    )));
                };
                path = Some(PathBuf::from(value));
            }
            value if value.starts_with("--config=") => {
                let path_value = &value["--config=".len()..];
                if path_value.is_empty() {
                    return Err(ControlError::Usage(format!(
                        "missing value for --config\n\n{}",
                        usage()
                    )));
                }
                path = Some(PathBuf::from(path_value));
            }
            other => {
                if path.is_some() {
                    return Err(ControlError::Usage(format!(
                        "unexpected extra argument: {other}\n\n{}",
                        usage()
                    )));
                }
                path = Some(PathBuf::from(other));
            }
        }
    }

    Ok(Command::ListThemes { path })
}

fn validate_target(path: &Path) -> Result<PathBuf, ControlError> {
    let canonical = fs::canonicalize(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ControlError::InvalidTarget(format!("target config does not exist: {}", path.display()))
        } else {
            ControlError::Io(err)
        }
    })?;

    let metadata = fs::metadata(&canonical)?;
    if !metadata.is_file() {
        return Err(ControlError::InvalidTarget(format!(
            "target config is not a file: {}",
            canonical.display()
        )));
    }

    Ok(canonical)
}

fn validate_config(path: &Path) -> Result<String, ControlError> {
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

fn doctor(path: &Path) -> Result<String, ControlError> {
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

fn list_themes(path: &Path) -> String {
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

fn switch_config(active_path: &Path, target_path: &Path) -> Result<String, ControlError> {
    if paths_match(active_path, target_path) {
        return Ok(format!(
            "active config already points to {}",
            target_path.display()
        ));
    }

    let Some(parent) = active_path.parent() else {
        return Err(ControlError::InvalidTarget(format!(
            "active config path has no parent directory: {}",
            active_path.display()
        )));
    };
    fs::create_dir_all(parent)?;

    maybe_backup_regular_file(active_path)?;

    let temp_link = parent.join(format!(".kwybarsctl-{}.tmp", std::process::id()));
    if temp_link.exists() {
        let _ = fs::remove_file(&temp_link);
    }

    create_symlink(target_path, &temp_link)?;
    fs::rename(&temp_link, active_path)?;

    Ok(format!(
        "switched active config {} -> {}",
        active_path.display(),
        target_path.display()
    ))
}

fn maybe_backup_regular_file(active_path: &Path) -> Result<(), ControlError> {
    let Ok(metadata) = fs::symlink_metadata(active_path) else {
        return Ok(());
    };

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Ok(());
    }

    let backup_path = active_path.with_file_name(format!(
        "{}.bak",
        active_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config.toml")
    ));
    if backup_path.exists() {
        return Ok(());
    }

    fs::copy(active_path, backup_path)?;
    Ok(())
}

fn paths_match(active_path: &Path, target_path: &Path) -> bool {
    fs::canonicalize(active_path)
        .ok()
        .is_some_and(|current| current == target_path)
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<(), ControlError> {
    use std::os::unix::fs::symlink;
    symlink(target, link)?;
    Ok(())
}

impl ControlError {
    fn usage_like(err: impl Display) -> Self {
        Self::InvalidTarget(err.to_string())
    }
}

#[cfg(not(unix))]
fn create_symlink(_target: &Path, _link: &Path) -> Result<(), ControlError> {
    Err(ControlError::InvalidTarget(
        "kwybarsctl switch-config requires Unix symlink support".to_owned(),
    ))
}

fn usage() -> String {
    "Usage:\n  kwybarsctl switch-config [--active <path>] <target-config.toml>\n  kwybarsctl validate-config [--config <path>]\n  kwybarsctl validate-config [path]\n  kwybarsctl doctor [--config <path>]\n  kwybarsctl doctor [path]\n  kwybarsctl list-themes [--config <path>]\n  kwybarsctl list-themes [path]\n  kwybarsctl --help\n\nCommands:\n  switch-config         Atomically switch the watched config path to another config file\n  validate-config       Validate config.toml, adjacent colors.toml, and configured theme\n  doctor                Report config/runtime environment status and likely setup issues\n  list-themes           List available user and built-in themes\n\nOptions:\n  -a, --active <path>   Active config path to update (default: normal Kwybars config path)\n  -c, --config <path>   Config path to validate/report/list themes for\n  -h, --help            Show this help message"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{Command, parse_args};
    use std::path::PathBuf;

    #[test]
    fn parses_switch_config_command() {
        let parsed = parse_args([
            "switch-config".into(),
            "--active".into(),
            "/tmp/current.toml".into(),
            "/tmp/alt.toml".into(),
        ]);

        let Ok(Command::SwitchConfig { target, active }) = parsed else {
            panic!("expected switch-config command");
        };
        assert_eq!(target, PathBuf::from("/tmp/alt.toml"));
        assert_eq!(active, Some(PathBuf::from("/tmp/current.toml")));
    }

    #[test]
    fn parses_help_command() {
        let parsed = parse_args(["--help".into()]);
        assert!(matches!(parsed, Ok(Command::Help)));
    }

    #[test]
    fn parses_validate_config_command() {
        let parsed = parse_args([
            "validate-config".into(),
            "--config".into(),
            "/tmp/custom.toml".into(),
        ]);

        let Ok(Command::ValidateConfig { path }) = parsed else {
            panic!("expected validate-config command");
        };
        assert_eq!(path, Some(PathBuf::from("/tmp/custom.toml")));
    }

    #[test]
    fn parses_doctor_command() {
        let parsed = parse_args(["doctor".into(), "/tmp/custom.toml".into()]);

        let Ok(Command::Doctor { path }) = parsed else {
            panic!("expected doctor command");
        };
        assert_eq!(path, Some(PathBuf::from("/tmp/custom.toml")));
    }

    #[test]
    fn parses_list_themes_command() {
        let parsed = parse_args(["list-themes".into(), "--config=/tmp/custom.toml".into()]);

        let Ok(Command::ListThemes { path }) = parsed else {
            panic!("expected list-themes command");
        };
        assert_eq!(path, Some(PathBuf::from("/tmp/custom.toml")));
    }
}
