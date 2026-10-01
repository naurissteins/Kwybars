//! default config file location

use std::ffi::OsString;
use std::path::PathBuf;

use crate::xdg;

/// environment variable that overrides the default config path
const CONFIG_ENV: &str = "KWYBARS_CONFIG";

/// no config path could be derived from the environment
#[derive(Debug, thiserror::Error)]
#[error(
    "could not determine the config path: set KWYBARS_CONFIG, XDG_CONFIG_HOME, or HOME, or pass --config"
)]
pub struct ConfigPathError;

pub fn default_path(env: &dyn Fn(&str) -> Option<OsString>) -> Result<PathBuf, ConfigPathError> {
    if let Some(path) = env_path(env) {
        return Ok(path);
    }
    xdg::base_dir(env, "XDG_CONFIG_HOME", ".config")
        .map(|dir| dir.join("kwybars").join("config.toml"))
        .ok_or(ConfigPathError)
}

/// the config path `$KWYBARS_CONFIG` names
pub fn env_path(env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    env(CONFIG_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::default_path;
    use crate::xdg::fake_env;

    #[test]
    fn override_variable_wins() {
        let env = fake_env(&[
            ("KWYBARS_CONFIG", "/tmp/active.toml"),
            ("XDG_CONFIG_HOME", "/cfg"),
        ]);
        assert_eq!(
            default_path(&env).ok(),
            Some(PathBuf::from("/tmp/active.toml"))
        );
    }

    #[test]
    fn empty_override_is_ignored() {
        let env = fake_env(&[("KWYBARS_CONFIG", ""), ("XDG_CONFIG_HOME", "/cfg")]);
        assert_eq!(
            default_path(&env).ok(),
            Some(PathBuf::from("/cfg/kwybars/config.toml"))
        );
    }

    #[test]
    fn falls_back_to_home() {
        let env = fake_env(&[("HOME", "/home/u")]);
        assert_eq!(
            default_path(&env).ok(),
            Some(PathBuf::from("/home/u/.config/kwybars/config.toml"))
        );
    }

    #[test]
    fn errors_without_any_location() {
        assert!(default_path(&fake_env(&[])).is_err());
    }
}
