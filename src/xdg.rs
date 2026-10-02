//! XDG base directory lookup

use std::ffi::OsString;
use std::path::PathBuf;

/// reads a variable from the real process environment
pub fn process_env(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// returns the directory named by the XDG variable `var`, falling back to
/// `$HOME/<home_fallback>`
pub fn base_dir(
    env: &dyn Fn(&str) -> Option<OsString>,
    var: &str,
    home_fallback: &str,
) -> Option<PathBuf> {
    let absolute = |name: &str| env(name).map(PathBuf::from).filter(|p| p.is_absolute());
    absolute(var).or_else(|| absolute("HOME").map(|home| home.join(home_fallback)))
}

/// builds a fake environment from `(name, value)` pairs
#[cfg(test)]
pub fn fake_env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + use<> {
    let pairs: Vec<(String, OsString)> = pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
        .collect();
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{base_dir, fake_env};

    #[test]
    fn prefers_absolute_xdg_variable() {
        let env = fake_env(&[("XDG_STATE_HOME", "/state"), ("HOME", "/home/u")]);
        assert_eq!(
            base_dir(&env, "XDG_STATE_HOME", ".local/state"),
            Some(PathBuf::from("/state"))
        );
    }

    #[test]
    fn ignores_empty_or_relative_xdg_variable() {
        for value in ["", "relative/state"] {
            let env = fake_env(&[("XDG_STATE_HOME", value), ("HOME", "/home/u")]);
            assert_eq!(
                base_dir(&env, "XDG_STATE_HOME", ".local/state"),
                Some(PathBuf::from("/home/u/.local/state"))
            );
        }
    }

    #[test]
    fn none_without_usable_variables() {
        let env = fake_env(&[("HOME", "")]);
        assert_eq!(base_dir(&env, "XDG_STATE_HOME", ".local/state"), None);
    }
}
