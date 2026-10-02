//! noticing config, `colors.toml`, theme, and image edits and loading them
//! off the main thread

mod scope;
mod targets;
#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::fs;
use std::io::{self, ErrorKind};
use std::os::fd::{AsFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use calloop::channel::Sender;
use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask, Watches};
use tracing::{debug, warn};

pub use scope::Scope;

use crate::config::{self, ConfigError, Loaded};
use crate::xdg;
use targets::targets;

const DEBOUNCE: Duration = Duration::from_millis(200);
const BUFFER_BYTES: usize = 4096;

const MASK: WatchMask = WatchMask::CLOSE_WRITE
    .union(WatchMask::MOVED_TO)
    .union(WatchMask::MOVED_FROM)
    .union(WatchMask::CREATE)
    .union(WatchMask::DELETE)
    .union(WatchMask::ONLYDIR);

/// a watched directory and the names in it that matter
type Watched = Vec<(WatchDescriptor, Vec<OsString>)>;

/// the files beside the config a load used or looked for
#[derive(Debug, Clone, Default)]
struct Sources {
    images: Vec<PathBuf>,
    /// where the configured theme may be, also while no such file exists
    themes: Vec<PathBuf>,
}

impl Sources {
    fn of(loaded: &Loaded) -> Self {
        Self {
            images: loaded.image_files(),
            themes: loaded.theme_candidates.clone(),
        }
    }
}

/// what a load worker sends back
pub struct Reloaded {
    pub result: Result<Loaded, ConfigError>,
    watched: Watched,
    sources: Sources,
}

/// watches the files a config is made of and loads it again when they change
pub struct Reloader {
    inotify: Inotify,
    buffer: Vec<u8>,
    watched: Watched,
    config_path: PathBuf,
    sources: Sources,
    due: Option<Instant>,
    loading: bool,
    again: bool,
    results: Sender<Reloaded>,
}

impl Reloader {
    /// starts watching what loaded was read from
    pub fn new(
        config_path: PathBuf,
        loaded: &Loaded,
        results: Sender<Reloaded>,
    ) -> io::Result<Self> {
        let inotify = Inotify::init()?;
        let sources = Sources::of(loaded);
        let watched = watch(&mut inotify.watches(), &config_path, &sources);
        Ok(Self {
            inotify,
            buffer: vec![0; BUFFER_BYTES],
            watched,
            config_path,
            sources,
            due: None,
            loading: false,
            again: false,
            results,
        })
    }

    /// a second descriptor for the event loop to poll
    pub fn fd(&self) -> io::Result<OwnedFd> {
        self.inotify.as_fd().try_clone_to_owned()
    }

    /// reads every pending event, true when one touched a watched file
    pub fn drain(&mut self) -> bool {
        let mut relevant = false;
        loop {
            let events = match self.inotify.read_events(&mut self.buffer) {
                Ok(events) => events,
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => {
                    warn!("could not read config file events: {err}");
                    break;
                }
            };
            for event in events {
                if event.mask.contains(EventMask::Q_OVERFLOW) {
                    relevant = true;
                }
                let Some(name) = event.name else {
                    continue;
                };
                let watched = self.watched.iter().any(|(wd, names)| {
                    *wd == event.wd && names.iter().any(|known| known.as_os_str() == name)
                });
                if watched {
                    debug!("config change: {}", name.to_string_lossy());
                    relevant = true;
                }
            }
        }
        relevant
    }

    pub fn changed(&mut self, now: Instant) -> Option<Instant> {
        let pending = self.due.is_some();
        let due = now + DEBOUNCE;
        self.due = Some(due);
        (!pending).then_some(due)
    }

    pub fn wake(&mut self, now: Instant) -> Option<Instant> {
        match self.due {
            Some(due) if due > now => Some(due),
            _ => {
                self.due = None;
                self.load();
                None
            }
        }
    }

    /// takes a worker's result and watches what it was read from
    pub fn finish(&mut self, reloaded: Reloaded) -> Result<Loaded, ConfigError> {
        self.loading = false;
        let mut watches = self.inotify.watches();
        for (wd, _) in &self.watched {
            if !reloaded.watched.iter().any(|(kept, _)| kept == wd) {
                // fails only when the directory is gone, which removed it too
                let _ = watches.remove(wd.clone());
            }
        }
        self.watched = reloaded.watched;
        self.sources = reloaded.sources;
        if self.again {
            self.again = false;
            self.load();
        }
        reloaded.result
    }

    /// reads and parses on a short-lived worker, one at a time
    fn load(&mut self) {
        if self.loading {
            self.again = true;
            return;
        }
        let path = self.config_path.clone();
        let previous = self.sources.clone();
        let mut watches = self.inotify.watches();
        let results = self.results.clone();
        let worker = std::thread::Builder::new()
            .name("kwybars-config".to_owned())
            .spawn(move || {
                let result = config::load(&path, &xdg::process_env);
                let sources = match &result {
                    Ok(loaded) => Sources::of(loaded),
                    Err(_) => previous,
                };
                let watched = watch(&mut watches, &path, &sources);
                // fails only when the main loop is gone
                let _ = results.send(Reloaded {
                    result,
                    watched,
                    sources,
                });
            });
        match worker {
            Ok(_) => self.loading = true,
            Err(err) => warn!("could not start loading the config: {err}"),
        }
    }
}

fn watch(watches: &mut Watches, config_path: &Path, sources: &Sources) -> Watched {
    let canonical = fs::canonicalize(config_path).ok();
    let canonical = canonical.as_deref().filter(|real| *real != config_path);
    targets(config_path, canonical, &sources.images, &sources.themes)
        .into_iter()
        .filter_map(|target| match watches.add(&target.dir, MASK) {
            Ok(wd) => Some((wd, target.names)),
            Err(err) => {
                debug!("not watching {}: {err}", target.dir.display());
                None
            }
        })
        .collect()
}
