//! the directories and file names a reload watches

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::config::COLORS_FILE;

/// a directory and the names in it that matter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub dir: PathBuf,
    pub names: Vec<OsString>,
}

pub fn targets(config: &Path, canonical: Option<&Path>, theme: Option<&Path>) -> Vec<Target> {
    let mut targets: Vec<Target> = Vec::new();
    for path in std::iter::once(config).chain(canonical) {
        add(&mut targets, path);
        add(&mut targets, &dir_of(path).join(COLORS_FILE));
    }
    if let Some(theme) = theme {
        add(&mut targets, theme);
    }
    targets
}

fn add(targets: &mut Vec<Target>, path: &Path) {
    let Some(name) = path.file_name() else {
        return;
    };
    let dir = dir_of(path);
    match targets.iter_mut().find(|target| target.dir == dir) {
        Some(target) if target.names.iter().any(|known| known == name) => {}
        Some(target) => target.names.push(name.to_owned()),
        None => targets.push(Target {
            dir,
            names: vec![name.to_owned()],
        }),
    }
}

/// the parent directory, `.` for a bare file name
fn dir_of(path: &Path) -> PathBuf {
    match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.to_owned(),
        _ => PathBuf::from("."),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::{Target, targets};

    fn target(dir: &str, names: &[&str]) -> Target {
        Target {
            dir: PathBuf::from(dir),
            names: names.iter().map(OsString::from).collect(),
        }
    }

    #[test]
    fn watches_the_link_and_its_target_with_colors_and_theme() {
        let found = targets(
            Path::new("/cfg/kwybars/current.toml"),
            Some(Path::new("/cfg/kwybars/custom/line.toml")),
            Some(Path::new("/cfg/kwybars/custom/themes/mocha.toml")),
        );
        assert_eq!(
            found,
            vec![
                target("/cfg/kwybars", &["current.toml", "colors.toml"]),
                target("/cfg/kwybars/custom", &["line.toml", "colors.toml"]),
                target("/cfg/kwybars/custom/themes", &["mocha.toml"]),
            ]
        );
    }

    #[test]
    fn a_theme_beside_the_config_shares_its_directory() {
        let found = targets(
            Path::new("/cfg/config.toml"),
            None,
            Some(Path::new("/cfg/theme.toml")),
        );
        assert_eq!(
            found,
            vec![target(
                "/cfg",
                &["config.toml", "colors.toml", "theme.toml"]
            )]
        );
    }

    #[test]
    fn a_bare_file_name_is_watched_in_the_working_directory() {
        let found = targets(Path::new("config.toml"), None, None);
        assert_eq!(found, vec![target(".", &["config.toml", "colors.toml"])]);
    }
}
