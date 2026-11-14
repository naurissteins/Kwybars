use std::ffi::OsStr;
use std::io;

use calloop::generic::Generic;
use calloop::{Dispatcher, Interest, Mode, PostAction};
use inotify::{EventMask, WatchMask};

use super::{App, Provider, Socket};
use crate::sweetbg::Connection;

impl Provider {
    pub(super) fn notice(&mut self) {
        let mut bytes = [0; 4096];
        loop {
            let events = match self.watcher.read_events(&mut bytes) {
                Ok(events) => events,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    tracing::warn!("could not watch sweetbg files: {error}");
                    break;
                }
            };
            for event in events {
                if event.mask.contains(EventMask::Q_OVERFLOW) {
                    self.dirty = true;
                    self.attempt = true;
                }
                let Some(name) = event.name else {
                    continue;
                };
                if name == OsStr::new("sweetbg.sock") {
                    if event
                        .mask
                        .intersects(EventMask::DELETE | EventMask::MOVED_FROM)
                    {
                        self.fail("socket removed");
                    }
                    if event
                        .mask
                        .intersects(EventMask::CREATE | EventMask::MOVED_TO | EventMask::ATTRIB)
                    {
                        self.attempt = true;
                    }
                } else if self
                    .known
                    .iter()
                    .any(|image| image.path.file_name() == Some(name))
                {
                    self.dirty = true;
                }
            }
        }
    }

    pub(super) fn watch_images(&mut self) {
        let mut watches = self.watcher.watches();
        for watch in self.image_watches.drain(..) {
            let _ = watches.remove(watch);
        }
        for image in &self.known {
            let Some(parent) = image.path.parent() else {
                continue;
            };
            if parent == self.runtime {
                continue;
            }
            if let Ok(watch) = watches.add(
                parent,
                WatchMask::CLOSE_WRITE
                    | WatchMask::MOVED_TO
                    | WatchMask::MOVED_FROM
                    | WatchMask::CREATE
                    | WatchMask::DELETE,
            ) && !self.image_watches.contains(&watch)
            {
                self.image_watches.push(watch);
            }
        }
    }
}

impl App {
    pub(in crate::app) fn sweetbg_poll(&mut self) {
        if !self.config.uses_sweetbg() {
            self.stop_sweetbg();
            self.sweetbg_failed = false;
            return;
        }
        if self.sweetbg.is_none() && !self.sweetbg_failed {
            self.sweetbg_epoch = self.sweetbg_epoch.wrapping_add(1);
            match Provider::start(self) {
                Ok(provider) => self.sweetbg = Some(provider),
                Err(error) => {
                    tracing::warn!("could not follow sweetbg: {error}");
                    self.sweetbg_failed = true;
                }
            }
        }
        let Some(provider) = &mut self.sweetbg else {
            return;
        };
        if provider.disconnected {
            if let Some(socket) = provider.socket.take() {
                self.handle.remove(socket.token);
            }
            provider.disconnected = false;
        }
        if provider.socket.is_none() && provider.attempt {
            provider.attempt = false;
            let opened =
                Connection::open(&provider.runtime.join("sweetbg.sock")).and_then(|connection| {
                    let dispatcher = Dispatcher::new(
                        Generic::new(connection.fd()?, Interest::BOTH, Mode::Level),
                        |_, _, app: &mut App| {
                            app.sweetbg_io();
                            Ok(PostAction::Continue)
                        },
                    );
                    let token = self
                        .handle
                        .register_dispatcher(dispatcher.clone())
                        .map_err(io::Error::other)?;
                    Ok(Socket {
                        connection,
                        dispatcher,
                        token,
                    })
                });
            match opened {
                Ok(socket) => {
                    provider.socket = Some(socket);
                    provider.warned = false;
                    tracing::info!("following sweetbg wallpapers");
                }
                Err(error) => provider.fail(error),
            }
        }
        if let Some(socket) = &provider.socket {
            let interest = if socket.connection.wants_write() {
                Interest::BOTH
            } else {
                Interest::READ
            };
            let changed = {
                let mut source = socket.dispatcher.as_source_mut();
                let changed = source.interest.writable != interest.writable;
                source.interest = interest;
                changed
            };
            if changed && let Err(error) = self.handle.update(&socket.token) {
                provider.fail(error);
            }
        }
        self.follow_backgrounds();
    }

    fn sweetbg_io(&mut self) {
        if let Some(provider) = &mut self.sweetbg {
            let events = provider
                .socket
                .as_mut()
                .map(|socket| socket.connection.receive());
            match events {
                Some(Ok(events)) => {
                    for event in events {
                        provider.event(event);
                    }
                }
                Some(Err(error)) => provider.fail(error),
                None => {}
            }
        }
        self.follow_backgrounds();
        self.render();
    }

    pub(in crate::app) fn stop_sweetbg(&mut self) {
        if let Some(provider) = self.sweetbg.take() {
            if let Some(socket) = &provider.socket {
                self.handle.remove(socket.token);
            }
            for token in provider.watch_tokens {
                self.handle.remove(token);
            }
            self.wayland.set_followed_backgrounds(&[], &[]);
        }
    }
}
