use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::ControlError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    SwitchConfig {
        target: PathBuf,
        active: Option<PathBuf>,
    },
    ImageOverlayMatch {
        wallpaper: PathBuf,
        overlay_dir: PathBuf,
        config: Option<PathBuf>,
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

pub fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Command, ControlError> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Ok(Command::Help);
    };

    match command.to_string_lossy().as_ref() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "switch-config" => parse_switch_config(args),
        "image-overlay" => parse_image_overlay(args),
        "validate-config" => parse_path_command(args, |path| Command::ValidateConfig { path }),
        "doctor" => parse_path_command(args, |path| Command::Doctor { path }),
        "list-themes" => parse_path_command(args, |path| Command::ListThemes { path }),
        other => Err(ControlError::Usage(format!(
            "unknown command: {other}\n\n{}",
            usage()
        ))),
    }
}

pub fn usage() -> String {
    "Usage:\n  kwybarsctl switch-config [--active <path>] <target-config.toml>\n  kwybarsctl image-overlay match --overlay-dir <dir> [--config <path>] <wallpaper>\n  kwybarsctl validate-config [--config <path>]\n  kwybarsctl validate-config [path]\n  kwybarsctl doctor [--config <path>]\n  kwybarsctl doctor [path]\n  kwybarsctl list-themes [--config <path>]\n  kwybarsctl list-themes [path]\n  kwybarsctl --help\n\nCommands:\n  switch-config         Atomically switch the watched config path to another config file\n  image-overlay match   Match a wallpaper basename to an overlay image and update config\n  validate-config       Validate config.toml, adjacent colors.toml, and configured theme\n  doctor                Report config/runtime environment status and likely setup issues\n  list-themes           List available user and built-in themes\n\nOptions:\n  -a, --active <path>   Active config path to update (default: normal Kwybars config path)\n  -c, --config <path>   Config path to update, validate, report, or list themes for\n      --overlay-dir <dir>\n                        Directory containing overlay images for image-overlay match\n  -h, --help            Show this help message"
        .to_owned()
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

fn parse_image_overlay(args: impl IntoIterator<Item = OsString>) -> Result<Command, ControlError> {
    let mut args = args.into_iter();
    let Some(subcommand) = args.next() else {
        return Err(ControlError::Usage(format!(
            "missing image-overlay subcommand\n\n{}",
            usage()
        )));
    };

    match subcommand.to_string_lossy().as_ref() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "match" => parse_image_overlay_match(args),
        other => Err(ControlError::Usage(format!(
            "unknown image-overlay subcommand: {other}\n\n{}",
            usage()
        ))),
    }
}

fn parse_image_overlay_match(
    args: impl IntoIterator<Item = OsString>,
) -> Result<Command, ControlError> {
    let mut args = args.into_iter();
    let mut config = None;
    let mut overlay_dir = None;
    let mut wallpaper = None;

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
                config = Some(PathBuf::from(value));
            }
            value if value.starts_with("--config=") => {
                let path = &value["--config=".len()..];
                if path.is_empty() {
                    return Err(ControlError::Usage(format!(
                        "missing value for --config\n\n{}",
                        usage()
                    )));
                }
                config = Some(PathBuf::from(path));
            }
            "--overlay-dir" => {
                let Some(value) = args.next() else {
                    return Err(ControlError::Usage(format!(
                        "missing value for --overlay-dir\n\n{}",
                        usage()
                    )));
                };
                overlay_dir = Some(PathBuf::from(value));
            }
            value if value.starts_with("--overlay-dir=") => {
                let path = &value["--overlay-dir=".len()..];
                if path.is_empty() {
                    return Err(ControlError::Usage(format!(
                        "missing value for --overlay-dir\n\n{}",
                        usage()
                    )));
                }
                overlay_dir = Some(PathBuf::from(path));
            }
            other => {
                if wallpaper.is_some() {
                    return Err(ControlError::Usage(format!(
                        "unexpected extra argument: {other}\n\n{}",
                        usage()
                    )));
                }
                wallpaper = Some(PathBuf::from(other));
            }
        }
    }

    let Some(overlay_dir) = overlay_dir else {
        return Err(ControlError::Usage(format!(
            "missing --overlay-dir\n\n{}",
            usage()
        )));
    };
    let Some(wallpaper) = wallpaper else {
        return Err(ControlError::Usage(format!(
            "missing wallpaper path\n\n{}",
            usage()
        )));
    };

    Ok(Command::ImageOverlayMatch {
        wallpaper,
        overlay_dir,
        config,
    })
}

fn parse_path_command(
    args: impl IntoIterator<Item = OsString>,
    build: impl FnOnce(Option<PathBuf>) -> Command,
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

    Ok(build(path))
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
    fn parses_image_overlay_match_command() {
        let parsed = parse_args([
            "image-overlay".into(),
            "match".into(),
            "--overlay-dir".into(),
            "/tmp/overlays".into(),
            "--config=/tmp/config.toml".into(),
            "/tmp/wallpapers/forest.jpg".into(),
        ]);

        let Ok(Command::ImageOverlayMatch {
            wallpaper,
            overlay_dir,
            config,
        }) = parsed
        else {
            panic!("expected image-overlay match command");
        };
        assert_eq!(wallpaper, PathBuf::from("/tmp/wallpapers/forest.jpg"));
        assert_eq!(overlay_dir, PathBuf::from("/tmp/overlays"));
        assert_eq!(config, Some(PathBuf::from("/tmp/config.toml")));
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
