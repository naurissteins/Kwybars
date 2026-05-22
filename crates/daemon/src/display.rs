use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct GraphicalEnvironment {
    wayland_display: Option<OsString>,
    display: Option<OsString>,
}

impl GraphicalEnvironment {
    pub(crate) fn detect() -> Option<Self> {
        let wayland_display =
            env::var_os("WAYLAND_DISPLAY").or_else(detect_wayland_display_from_runtime);
        let display = env::var_os("DISPLAY");

        if wayland_display.is_none() && display.is_none() {
            return None;
        }

        Some(Self {
            wayland_display,
            display,
        })
    }

    pub(crate) fn apply_to(&self, command: &mut Command) {
        if let Some(value) = &self.wayland_display {
            command.env("WAYLAND_DISPLAY", value);
        }
        if let Some(value) = &self.display {
            command.env("DISPLAY", value);
        }
    }
}

fn detect_wayland_display_from_runtime() -> Option<OsString> {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR")?;
    detect_wayland_display_in(Path::new(&runtime_dir))
}

fn detect_wayland_display_in(runtime_dir: &Path) -> Option<OsString> {
    let entries = fs::read_dir(runtime_dir).ok()?;
    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        candidates.push((file_name, file_type.is_socket()));
    }

    select_wayland_display_candidate(candidates)
}

fn select_wayland_display_candidate(
    candidates: impl IntoIterator<Item = (OsString, bool)>,
) -> Option<OsString> {
    let mut candidates = candidates
        .into_iter()
        .filter_map(|(file_name, is_socket)| {
            let name = file_name.to_str()?;
            if is_socket && name.starts_with("wayland-") && !name.ends_with(".lock") {
                Some(file_name)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    candidates.sort();
    candidates.into_iter().next()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::PathBuf;

    use super::{detect_wayland_display_in, select_wayland_display_candidate};

    #[test]
    fn selects_first_wayland_socket_candidate() {
        let detected = select_wayland_display_candidate([
            (std::ffi::OsString::from("wayland-1.lock"), true),
            (std::ffi::OsString::from("wayland-2"), true),
            (std::ffi::OsString::from("wayland-1"), true),
            (std::ffi::OsString::from("not-wayland"), true),
            (std::ffi::OsString::from("wayland-0"), false),
        ]);

        assert_eq!(detected.as_deref(), Some(std::ffi::OsStr::new("wayland-1")));
    }

    #[test]
    fn ignores_non_socket_wayland_entries() -> io::Result<()> {
        let runtime_dir = unique_temp_dir("kwybars-wayland-nonsocket");
        fs::create_dir_all(&runtime_dir)?;
        fs::write(runtime_dir.join("wayland-1"), "")?;

        let detected = detect_wayland_display_in(&runtime_dir);
        let _ = fs::remove_dir_all(&runtime_dir);

        assert_eq!(detected, None);
        Ok(())
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{name}-{}", std::process::id()))
    }
}
