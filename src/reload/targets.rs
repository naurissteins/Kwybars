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

/// `files` exist; a file in `wanted` may appear later, with its directory
pub fn targets(
    config: &Path,
    canonical: Option<&Path>,
    files: &[PathBuf],
    wanted: &[PathBuf],
) -> Vec<Target> {
    let mut targets: Vec<Target> = Vec::new();
    for path in std::iter::once(config).chain(canonical) {
        add(&mut targets, path);
        add(&mut targets, &dir_of(path).join(COLORS_FILE));
    }
    // the config directory created or recreated is seen from its parent
    add(&mut targets, &dir_of(config));
    for file in files {
        add(&mut targets, file);
    }
    for file in wanted {
        add(&mut targets, file);
        // creating the directory is seen from its parent
        add(&mut targets, &dir_of(file));
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
    fn watches_the_link_and_its_target_with_colors_theme_and_image() {
        let found = targets(
            Path::new("/cfg/kwybars/current.toml"),
            Some(Path::new("/cfg/kwybars/custom/line.toml")),
            &[PathBuf::from("/cfg/kwybars/overlays/02.jpg")],
            &[
                PathBuf::from("/cfg/kwybars/themes/mocha.toml"),
                PathBuf::from("/cfg/kwybars/custom/themes/mocha.toml"),
                PathBuf::from("/usr/share/kwybars/themes/mocha.toml"),
            ],
        );
        assert_eq!(
            found,
            vec![
                target("/cfg/kwybars", &["current.toml", "colors.toml", "themes"]),
                target(
                    "/cfg/kwybars/custom",
                    &["line.toml", "colors.toml", "themes"]
                ),
                target("/cfg", &["kwybars"]),
                target("/cfg/kwybars/overlays", &["02.jpg"]),
                target("/cfg/kwybars/themes", &["mocha.toml"]),
                target("/cfg/kwybars/custom/themes", &["mocha.toml"]),
                target("/usr/share/kwybars/themes", &["mocha.toml"]),
                target("/usr/share/kwybars", &["themes"]),
            ]
        );
    }

    #[test]
    fn a_theme_beside_the_config_shares_its_directory() {
        let found = targets(
            Path::new("/cfg/config.toml"),
            None,
            &[PathBuf::from("/cfg/theme.toml")],
            &[],
        );
        assert_eq!(
            found,
            vec![
                target("/cfg", &["config.toml", "colors.toml", "theme.toml"]),
                target("/", &["cfg"]),
            ]
        );
    }

    #[test]
    fn a_bare_file_name_is_watched_in_the_working_directory() {
        let found = targets(Path::new("config.toml"), None, &[], &[]);
        assert_eq!(found, vec![target(".", &["config.toml", "colors.toml"])]);
    }
}
