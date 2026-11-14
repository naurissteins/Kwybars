//! optional sweetbg following, with socket/image notifications and short-lived loads

mod apply;
mod io;
mod load;

use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::sync::Arc;

use calloop::channel::{self, Sender};
use calloop::generic::Generic;
use calloop::{Dispatcher, Interest, Mode, RegistrationToken};
use inotify::{Inotify, WatchMask};

use super::App;
use crate::config::LoadedImage;
use crate::render::image::Encoded;
use crate::sweetbg::{Connection, Event, Output};

struct Socket {
    connection: Connection,
    dispatcher: Dispatcher<'static, Generic<OwnedFd>, App>,
    token: RegistrationToken,
}

pub(super) struct Provider {
    epoch: u64,
    runtime: PathBuf,
    watcher: Inotify,
    watch_tokens: Vec<RegistrationToken>,
    socket: Option<Socket>,
    disconnected: bool,
    attempt: bool,
    revision: u32,
    desired: Vec<Output>,
    owned: Vec<String>,
    claiming: Option<String>,
    rejected: Vec<String>,
    images: Vec<(String, Arc<Encoded>)>,
    known: Vec<LoadedImage<Encoded>>,
    selection: u64,
    request: u64,
    loading: Option<u64>,
    dirty: bool,
    results: Sender<load::Ready>,
    image_watches: Vec<inotify::WatchDescriptor>,
    warned: bool,
}

impl Provider {
    fn start(app: &mut App) -> std::io::Result<Self> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "sweetbg requires an absolute XDG_RUNTIME_DIR",
                )
            })?;
        let watcher = Inotify::init()?;
        watcher.watches().add(
            &runtime,
            WatchMask::CREATE
                | WatchMask::DELETE
                | WatchMask::MOVED_TO
                | WatchMask::MOVED_FROM
                | WatchMask::ATTRIB
                | WatchMask::CLOSE_WRITE,
        )?;
        let watch_fd = watcher.as_fd().try_clone_to_owned()?;
        let (results, channel) = channel::channel();
        let channel_token = app
            .handle
            .insert_source(channel, |event, (), app| {
                if let channel::Event::Msg(ready) = event {
                    app.sweetbg_loaded(ready);
                }
            })
            .map_err(|error| std::io::Error::other(error.error))?;
        let watch_token = match app.handle.insert_source(
            Generic::new(watch_fd, Interest::READ, Mode::Level),
            |_, _, app| {
                if let Some(provider) = &mut app.sweetbg {
                    provider.notice();
                }
                app.follow_backgrounds();
                Ok(calloop::PostAction::Continue)
            },
        ) {
            Ok(token) => token,
            Err(error) => {
                app.handle.remove(channel_token);
                return Err(std::io::Error::other(error.error));
            }
        };
        Ok(Self {
            epoch: app.sweetbg_epoch,
            runtime,
            watcher,
            watch_tokens: vec![channel_token, watch_token],
            socket: None,
            disconnected: false,
            attempt: true,
            revision: 0,
            desired: Vec::new(),
            owned: Vec::new(),
            claiming: None,
            rejected: Vec::new(),
            images: Vec::new(),
            known: Vec::new(),
            selection: 0,
            request: 0,
            loading: None,
            dirty: false,
            results,
            image_watches: Vec::new(),
            warned: false,
        })
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::Snapshot { revision, outputs } => {
                self.revision = revision;
                self.rejected.clear();
                for output in &outputs {
                    if !output.path.as_os_str().is_empty() && !output.supported() {
                        tracing::warn!(
                            "sweetbg background on {}: only absolute-path cover images are supported (fit {})",
                            output.name,
                            output.fit
                        );
                    }
                }
                self.desired = outputs;
                self.images.retain(|(name, _)| {
                    self.desired
                        .iter()
                        .any(|output| output.name == *name && output.supported())
                });
                self.dirty = true;
            }
            Event::Ack {
                revision,
                name,
                accepted,
            } => {
                if self.claiming.as_ref() == Some(&name) {
                    self.claiming = None;
                    if accepted {
                        if !self.owned.contains(&name) {
                            self.owned.push(name);
                        }
                    } else if revision == self.revision {
                        tracing::warn!(
                            "sweetbg handoff on {name} was rejected; waiting for the next wallpaper update"
                        );
                        self.rejected.push(name);
                    }
                }
            }
        }
    }

    fn fail(&mut self, error: impl std::fmt::Display) {
        if !self.warned {
            tracing::warn!("sweetbg subscription unavailable: {error}; waiting for its socket");
            self.warned = true;
        }
        self.disconnected = true;
        self.attempt = false;
        self.request = self.request.wrapping_add(1);
        self.desired.clear();
        self.images.clear();
        self.owned.clear();
        self.known.clear();
        self.claiming = None;
        self.revision = 0;
    }

    fn load(&mut self, selected: &[String]) {
        if self.socket.is_none() || self.revision == 0 || !self.dirty || self.loading.is_some() {
            return;
        }
        self.dirty = false;
        self.request = self.request.wrapping_add(1);
        let (epoch, request) = (self.epoch, self.request);
        let desired = self
            .desired
            .iter()
            .filter(|output| output.supported() && selected.contains(&output.name))
            .cloned()
            .collect();
        let previous = self.known.clone();
        let results = self.results.clone();
        match std::thread::Builder::new()
            .name("kwybars-sweetbg".into())
            .spawn(move || {
                let _ = results.send(load::load(epoch, request, desired, previous));
            }) {
            Ok(_) => self.loading = Some(request),
            Err(error) => tracing::warn!("could not load sweetbg wallpaper: {error}"),
        }
    }
}

use std::os::fd::AsFd;
