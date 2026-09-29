//! startup, run, and shutdown of the overlay

mod animation;
pub mod debug;
mod error;
mod logging;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use calloop::EventLoop;
use calloop::ping::make_ping;
use calloop::signals::{Signal, Signals};
use tracing::{info, warn};

pub use error::AppError;

use crate::audio::capture::{Capture, CaptureSettings};
use crate::audio::dynamics::{Dynamics, DynamicsConfig};
use crate::audio::motion::Motion;
use crate::audio::spectrum::SpectrumConfig;
use crate::wayland::Wayland;
use crate::{config, xdg};
use animation::Animation;

/// options for running the overlay
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunOptions {
    /// config path from `--config`; `None` means the default location
    pub config_path: Option<PathBuf>,
}

/// runs the overlay until SIGINT or SIGTERM
pub fn run(options: RunOptions) -> Result<(), AppError> {
    // must run before any thread is spawned, see `logging::init`
    let log = logging::init(&xdg::process_env);
    info!("kwybars {} starting", env!("CARGO_PKG_VERSION"));
    log.report();

    let loaded = load_config(&options)?;
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
    let mut app = App {
        running: true,
        animation: Animation::new(motion, frame_time),
        wayland,
    };
    let handle = event_loop.handle();
    handle
        .insert_source(signals, |event, (), app| {
            info!("received {:?}, shutting down", event.signal());
            app.running = false;
        })
        .map_err(|err| err.error)?;
    handle
        .insert_source(wake, |(), (), app| {
            app.animation.wake();
            app.render();
        })
        .map_err(|err| err.error)?;
    app.wayland.insert_source(queue, &handle, App::render)?;

    let result = dispatch(&mut event_loop, &mut app);
    // teardown order: the audio thread, then surfaces and buffers, then the
    // connection, which closes when the loop and its wayland source drop
    capture.stop();
    app.wayland.shutdown();
    info!("kwybars stopped");
    result
}

/// state the main loop callbacks share
struct App {
    running: bool,
    animation: Animation,
    wayland: Wayland,
}

impl App {
    /// steps the bars if a frame is due and lets ready surfaces draw it
    fn render(&mut self) {
        let frame = self.animation.frame(Instant::now());
        let drawn = self.wayland.render(&frame);
        self.animation.drawn(drawn);
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
    app.animation.wake();
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

/// blocks SIGINT and SIGTERM in this thread and returns a source for them;
/// threads spawned afterwards inherit the mask, so the source is the only
/// receiver
fn block_signals() -> Result<Signals, AppError> {
    Ok(Signals::new(&[Signal::SIGINT, Signal::SIGTERM])?)
}

/// time between frames at `framerate` frames per second
fn frame_time(framerate: u32) -> Duration {
    Duration::from_secs(1) / framerate.max(1)
}

/// resolves and loads the config, logging where everything came from
fn load_config(options: &RunOptions) -> Result<config::Loaded, AppError> {
    let config_path = match &options.config_path {
        Some(path) => path.clone(),
        None => config::default_path(&xdg::process_env)?,
    };
    let loaded = config::load(&config_path, &xdg::process_env)?;
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
