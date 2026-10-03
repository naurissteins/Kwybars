//! logging to stderr and to a log file

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use time::UtcOffset;
use time::macros::format_description;
use tracing::level_filters::LevelFilter;
use tracing::{info, warn};
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::{self, time::OffsetTime};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::xdg;

/// environment variable naming an explicit log file
const LOG_FILE_ENV: &str = "KWYBARS_LOG_FILE";
const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// outcome of logging setup, reported once the subscriber is installed
pub struct LogSetup {
    file: FileLog,
    invalid_filter: Option<String>,
}

enum FileLog {
    Enabled(PathBuf),
    Failed { path: PathBuf, error: io::Error },
    NoLocation,
}

/// installs the global subscriber
pub fn init(env: &dyn Fn(&str) -> Option<OsString>) -> LogSetup {
    let (filter, invalid_filter) = filter_from_env(env);
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let timer = OffsetTime::new(
        offset,
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"),
    );

    let stderr_layer = fmt::layer()
        .with_timer(timer.clone())
        .with_target(false)
        .with_ansi(io::stderr().is_terminal())
        .with_writer(io::stderr);

    let (file, file_layer) = match log_file_path(env) {
        None => (FileLog::NoLocation, None),
        Some(path) => match open_log_file(&path) {
            Ok(handle) => {
                let layer = fmt::layer()
                    .with_timer(timer)
                    .with_target(false)
                    .with_ansi(false)
                    .with_writer(Mutex::new(handle));
                (FileLog::Enabled(path), Some(layer))
            }
            Err(error) => (FileLog::Failed { path, error }, None),
        },
    };

    // fails only if a global subscriber is already installed, which never
    // happens in the binary; the existing subscriber then stays in charge
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer)
        .with(file_layer)
        .try_init();

    LogSetup {
        file,
        invalid_filter,
    }
}

impl LogSetup {
    /// logs where file logging goes and any setup problems
    pub fn report(&self) {
        if let Some(message) = &self.invalid_filter {
            warn!("{message}");
        }
        match &self.file {
            FileLog::Enabled(path) => info!("log file: {}", path.display()),
            FileLog::Failed { path, error } => warn!(
                "could not open log file {}: {error}; logging to stderr only",
                path.display()
            ),
            FileLog::NoLocation => warn!(
                "no log file location (set {LOG_FILE_ENV}, XDG_STATE_HOME, or HOME); logging to stderr only"
            ),
        }
    }
}

/// reads the filter from KWYBARS_LOG, then RUST_LOG, defaulting to info
fn filter_from_env(env: &dyn Fn(&str) -> Option<OsString>) -> (Targets, Option<String>) {
    let default = Targets::new().with_default(LevelFilter::INFO);
    let raw = env("KWYBARS_LOG")
        .or_else(|| env("RUST_LOG"))
        .map(|value| value.to_string_lossy().trim().to_owned())
        .filter(|value| !value.is_empty());
    let Some(raw) = raw else {
        return (default, None);
    };
    match raw.parse::<Targets>() {
        Ok(targets) if targets.default_level().is_none() => {
            (targets.with_default(LevelFilter::INFO), None)
        }
        Ok(targets) => (targets, None),
        Err(err) => (
            default,
            Some(format!("ignoring invalid log filter {raw:?}: {err}")),
        ),
    }
}

pub fn log_file_path(env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    if let Some(path) = env(LOG_FILE_ENV).filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(path));
    }
    xdg::base_dir(env, "XDG_STATE_HOME", ".local/state")
        .map(|dir| dir.join("kwybars").join("kwybars.log"))
}

fn open_log_file(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    move_aside_if_over(path, MAX_LOG_BYTES);
    OpenOptions::new().create(true).append(true).open(path)
}

fn move_aside_if_over(path: &Path, max: u64) {
    if fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() > max) {
        let mut old = path.as_os_str().to_owned();
        old.push(".old");
        let _ = fs::rename(path, old);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tracing::Level;

    use super::{filter_from_env, log_file_path, move_aside_if_over};
    use crate::config::tests::TempDir;
    use crate::xdg::fake_env;

    #[test]
    fn a_large_log_is_moved_aside_and_a_small_one_kept() {
        let dir = TempDir::new("log-size");
        let log = dir.write("kwybars.log", "0123456789");
        move_aside_if_over(&log, 10);
        assert!(log.exists());

        dir.write("kwybars.log.old", "older");
        dir.write("kwybars.log", "0123456789a");
        move_aside_if_over(&log, 10);
        assert!(!log.exists());
        let old = std::fs::read_to_string(dir.path().join("kwybars.log.old"));
        assert_eq!(old.ok().as_deref(), Some("0123456789a"));
    }

    #[test]
    fn explicit_log_file_wins() {
        let env = fake_env(&[("KWYBARS_LOG_FILE", "/tmp/k.log"), ("HOME", "/home/u")]);
        assert_eq!(log_file_path(&env), Some(PathBuf::from("/tmp/k.log")));
    }

    #[test]
    fn log_file_defaults_to_state_dir() {
        let env = fake_env(&[("HOME", "/home/u")]);
        assert_eq!(
            log_file_path(&env),
            Some(PathBuf::from("/home/u/.local/state/kwybars/kwybars.log"))
        );
    }

    #[test]
    fn kwybars_log_takes_precedence_over_rust_log() {
        let (targets, invalid) = filter_from_env(&fake_env(&[
            ("KWYBARS_LOG", "debug"),
            ("RUST_LOG", "=nope="),
        ]));
        assert!(invalid.is_none());
        assert!(targets.would_enable("kwybars", &Level::DEBUG));
    }

    #[test]
    fn invalid_filter_falls_back_to_info_and_is_reported() {
        let (targets, invalid) = filter_from_env(&fake_env(&[("RUST_LOG", "=nope=")]));
        assert!(invalid.is_some());
        assert!(targets.would_enable("kwybars", &Level::INFO));
        assert!(!targets.would_enable("kwybars", &Level::DEBUG));
    }

    #[test]
    fn foreign_rust_log_does_not_silence_kwybars() {
        let (targets, _) = filter_from_env(&fake_env(&[(
            "RUST_LOG",
            "sweets=info,sweets::shell=debug",
        )]));
        assert!(targets.would_enable("kwybars", &Level::INFO));
        assert!(targets.would_enable("sweets::shell", &Level::DEBUG));
    }

    #[test]
    fn explicit_off_is_respected() {
        let (targets, _) = filter_from_env(&fake_env(&[("KWYBARS_LOG", "off")]));
        assert!(!targets.would_enable("kwybars", &Level::ERROR));
    }
}
