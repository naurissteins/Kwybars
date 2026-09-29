//! startup, run, and shutdown of the overlay

mod animation;
pub mod debug;
mod error;
mod logging;
mod reload;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use calloop::ping::make_ping;
use calloop::signals::{Signal, Signals};
use calloop::timer::{TimeoutAction, Timer};
use calloop::{EventLoop, LoopHandle, RegistrationToken};
use tracing::{info, warn};

pub use error::AppError;

use crate::activity::ActivityTracker;
use crate::audio::capture::{Capture, CaptureSettings};
use crate::audio::dynamics::{Dynamics, DynamicsConfig};
use crate::audio::motion::Motion;
use crate::audio::spectrum::SpectrumConfig;
use crate::config::{Config, Theme};
use crate::reload::Reloader;
use crate::wayland::Wayland;
use crate::{config, xdg};
use animation::Animation;

/// options for running the overlay
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunOptions {
    /// config path from `--config`, `None` means the default location
    pub config_path: Option<PathBuf>,
}

/// runs the overlay until SIGINT or SIGTERM
pub fn run(options: RunOptions) -> Result<(), AppError> {
    // must run before any thread is spawned, see `logging::init`
    let log = logging::init(&xdg::process_env);
    info!("kwybars {} starting", env!("CARGO_PKG_VERSION"));
    log.report();

    let config_path = config_path(&options)?;
    let loaded = load_config(&config_path)?;
    let config = &loaded.config;
    info!(
        "config: layout {:?}, {} bars at {} fps, {} output override(s)",
        config.visualizer.layout,
        config.visualizer.bars,
        config.visualizer.framerate,
        config.overlay.outputs.len()
    );

    let signals = block_signals()?;
    // before the audio thread starts, so a compositor without layer shell fails fast
    let theme = loaded.theme.as_ref().map(|loaded| loaded.theme.clone());
    let (wayland, queue) = Wayland::connect(config.clone(), theme)?;
    let (waker, wake) = make_ping().map_err(calloop::Error::from)?;
    let frame_time = frame_time(config.visualizer.framerate);
    let spectrum = SpectrumConfig::from_config(config);
    let dynamics = Dynamics::new(spectrum.bars, DynamicsConfig::from_config(config));
    let capture = Capture::spawn(
        CaptureSettings {
            interval: frame_time,
            spectrum,
        },
        Some(waker),
    )?;
    let motion = Motion::new(Arc::clone(capture.frames()), dynamics, frame_time);

    let mut event_loop: EventLoop<'static, App> = EventLoop::try_new()?;
    let handle = event_loop.handle();
    let reloader = reload::start(&handle, config_path, &loaded);
    let mut app = App {
        running: true,
        animation: Animation::new(motion, frame_time),
        activity: ActivityTracker::new(&config.activity),
        timer: None,
        handle: handle.clone(),
        wayland,
        capture,
        reloader,
        config: loaded.config,
        theme: loaded.theme.map(|loaded| loaded.theme),
        warnings: loaded.warnings,
    };
    handle
        .insert_source(signals, |event, (), app| {
            info!("received {:?}, shutting down", event.signal());
            app.running = false;
        })
        .map_err(|err| err.error)?;
    handle
        .insert_source(wake, |(), (), app| app.render())
        .map_err(|err| err.error)?;
    app.wayland.insert_source(queue, &handle, App::render)?;

    let result = dispatch(&mut event_loop, &mut app);
    // teardown order: the audio thread, then surfaces and buffers, then the
    // connection, which closes when the loop and its wayland source drop
    app.capture.shutdown();
    app.wayland.shutdown();
    info!("kwybars stopped");
    result
}

/// state the main loop callbacks share
struct App {
    running: bool,
    animation: Animation,
    activity: ActivityTracker,
    /// the pending activity deadline timer and when it fires
    timer: Option<(RegistrationToken, Instant)>,
    handle: LoopHandle<'static, App>,
    wayland: Wayland,
    capture: Capture,
    reloader: Option<Reloader>,
    config: Config,
    theme: Option<Theme>,
    warnings: Vec<String>,
}

impl App {
    /// follows the audio level, shows or hides the surfaces, steps the bars
    /// if a frame is due, and lets ready surfaces draw it
    fn render(&mut self) {
        let now = Instant::now();
        let level = self.animation.level();
        if self.activity.update(now, level) {
            info!(
                "audio {}",
                if self.activity.is_active() {
                    "active, showing"
                } else {
                    "inactive, hiding"
                }
            );
        }
        let presence = self.wayland.set_active(self.activity.is_active(), now);
        let frame = self.animation.frame(now, presence);
        let drawn = self.wayland.render(&frame);
        self.animation.drawn(drawn);
        self.watch_deadline();
    }

    fn watch_deadline(&mut self) {
        let Some(deadline) = self.activity.deadline() else {
            return;
        };
        if self.timer.is_some_and(|(_, at)| at <= deadline) {
            return;
        }
        if let Some((token, _)) = self.timer.take() {
            self.handle.remove(token);
        }
        let timer = Timer::from_deadline(deadline);
        match self.handle.insert_source(timer, |_, (), app| {
            app.timer = None;
            app.render();
            TimeoutAction::Drop
        }) {
            Ok(token) => self.timer = Some((token, deadline)),
            Err(err) => warn!("could not wait for the activity delay: {}", err.error),
        }
    }
}

impl AsMut<Wayland> for App {
    fn as_mut(&mut self) -> &mut Wayland {
        &mut self.wayland
    }
}

/// runs the loop until a signal asks to stop
fn dispatch(event_loop: &mut EventLoop<'static, App>, app: &mut App) -> Result<(), AppError> {
    // a frame may have arrived before anyone asked to be woken
    app.render();
    while app.running {
        if let Err(err) = event_loop.dispatch(None, app) {
            // name the compositor when the connection is what failed
            app.wayland.check_connection()?;
            return Err(err.into());
        }
    }
    Ok(())
}

fn block_signals() -> Result<Signals, AppError> {
    Ok(Signals::new(&[Signal::SIGINT, Signal::SIGTERM])?)
}

/// time between frames at `framerate` frames per second
fn frame_time(framerate: u32) -> Duration {
    Duration::from_secs(1) / framerate.max(1)
}

/// `--config`, else the default location
fn config_path(options: &RunOptions) -> Result<PathBuf, AppError> {
    Ok(match &options.config_path {
        Some(path) => path.clone(),
        None => config::default_path(&xdg::process_env)?,
    })
}

/// loads the config, logging where everything came from
fn load_config(config_path: &Path) -> Result<config::Loaded, AppError> {
    let loaded = config::load(config_path, &xdg::process_env)?;
    match loaded.source {
        config::Source::File => info!("config path: {} (found)", config_path.display()),
        config::Source::Defaults => info!(
            "config path: {} (not found, using built-in defaults)",
            config_path.display()
        ),
    }
    for warning in &loaded.warnings {
        warn!("{warning}");
    }
    if let Some(path) = &loaded.colors_path {
        info!("colors: {}", path.display());
    }
    if let Some(loaded_theme) = &loaded.theme {
        match &loaded_theme.origin {
            config::ThemeOrigin::File(path) => info!("theme: {}", path.display()),
            config::ThemeOrigin::BuiltIn => {
                info!("theme: {} (built-in)", loaded_theme.theme.name);
            }
        }
    }
    Ok(loaded)
}
