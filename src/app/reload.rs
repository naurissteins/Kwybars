//! applying config edits while running

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use calloop::channel::{self, Channel};
use calloop::generic::Generic;
use calloop::timer::{TimeoutAction, Timer};
use calloop::{Interest, LoopHandle, Mode, PostAction};
use tracing::{error, info, warn};

use super::{App, frame_time, image};
use crate::audio::capture::CaptureSettings;
use crate::audio::dynamics::{Dynamics, DynamicsConfig};
use crate::audio::motion::Motion;
use crate::audio::spectrum::SpectrumConfig;
use crate::config::{Loaded, Source};
use crate::reload::{Reloaded, Reloader, Scope};

pub(super) fn start(
    handle: &LoopHandle<'static, App>,
    config_path: PathBuf,
    loaded: &Loaded,
) -> Option<Reloader> {
    let (results, channel) = channel::channel();
    let started = Reloader::new(config_path, loaded, results).and_then(|reloader| {
        register(handle, &reloader, channel)?;
        Ok(reloader)
    });
    match started {
        Ok(reloader) => Some(reloader),
        Err(err) => {
            warn!("config changes will not be picked up: {err}");
            None
        }
    }
}

fn register(
    handle: &LoopHandle<'static, App>,
    reloader: &Reloader,
    channel: Channel<Reloaded>,
) -> std::io::Result<()> {
    let events = Generic::new(reloader.fd()?, Interest::READ, Mode::Level);
    let failed = |err| std::io::Error::other(err);
    handle
        .insert_source(events, |_, _, app| {
            if app.reloader.as_mut().is_some_and(Reloader::drain) {
                app.reload_soon();
            }
            Ok(PostAction::Continue)
        })
        .map_err(|err| failed(err.error))?;
    handle
        .insert_source(channel, |event, (), app| {
            if let channel::Event::Msg(reloaded) = event {
                app.reloaded(reloaded);
            }
        })
        .map_err(|err| failed(err.error))?;
    Ok(())
}

impl App {
    /// loads once the files have been quiet for a moment
    fn reload_soon(&mut self) {
        let Some(due) = self
            .reloader
            .as_mut()
            .and_then(|reloader| reloader.changed(Instant::now()))
        else {
            return;
        };
        let timer = Timer::from_deadline(due);
        let inserted = self.handle.insert_source(timer, |_, (), app| {
            match app
                .reloader
                .as_mut()
                .and_then(|reloader| reloader.wake(Instant::now()))
            {
                Some(later) => TimeoutAction::ToInstant(later),
                None => TimeoutAction::Drop,
            }
        });
        if let Err(err) = inserted {
            warn!("could not schedule the config reload: {}", err.error);
        }
    }

    fn reloaded(&mut self, reloaded: Reloaded) {
        let Some(reloader) = self.reloader.as_mut() else {
            return;
        };
        match reloader.finish(reloaded) {
            Ok(loaded) => self.apply(loaded),
            Err(err) => error!("config reload failed, keeping the running config: {err}"),
        }
    }

    /// rebuilds only what the new config changes
    fn apply(&mut self, loaded: Loaded) {
        for warning in &loaded.warnings {
            if !self.warnings.contains(warning) {
                warn!("{warning}");
            }
        }
        self.warnings = loaded.warnings;
        if loaded.source == Source::Defaults {
            warn!("the config file is gone, using built-in defaults");
        }
        let config = loaded.config;
        let theme = loaded.theme.map(|loaded| loaded.theme);
        let image = image::overlay(&config, loaded.image, self.image.as_ref());
        let scope = Scope::between(
            (&self.config, self.theme.as_ref(), self.image.as_ref()),
            (&config, theme.as_ref(), image.as_ref()),
        );
        if scope.is_empty() {
            info!("config reloaded, nothing changed");
            return;
        }
        info!("config reloaded, updating {scope}");

        let frame_time = frame_time(config.visualizer.framerate);
        let dynamics = DynamicsConfig::from_config(&config);
        if scope.analysis {
            let spectrum = SpectrumConfig::from_config(&config);
            let bars = spectrum.bars;
            let settings = CaptureSettings {
                interval: frame_time,
                spectrum,
            };
            if self.capture.reconfigure(settings) {
                let frames = Arc::clone(self.capture.frames());
                let motion = Motion::new(frames, Dynamics::new(bars, dynamics.clone()), frame_time);
                self.animation.set_motion(motion);
            }
        }
        if scope.motion {
            self.animation.set_frame_time(frame_time);
            self.animation.motion_mut().set_dynamics(dynamics);
        }
        if scope.activity {
            self.activity.reconfigure(&config.activity);
        }
        if scope.surfaces {
            self.wayland
                .reconfigure(config.clone(), theme.clone(), Instant::now());
        }
        if scope.image {
            self.wayland.set_image(image.clone());
        }
        self.config = config;
        self.theme = theme;
        self.image = image;
        self.render();
    }
}
