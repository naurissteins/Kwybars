//! `kwybars debug ...` runners

use std::sync::Arc;
use std::time::{Duration, Instant};

use calloop::EventLoop;
use calloop::signals::Signals;
use calloop::timer::{TimeoutAction, Timer};

use super::{AppError, RunOptions, block_signals, frame_time, load_config, logging};
use crate::audio::capture::{Capture, CaptureSettings, StatusSnapshot};
use crate::audio::dynamics::{Dynamics, DynamicsConfig};
use crate::audio::motion::Motion;
use crate::audio::spectrum::SpectrumConfig;
use crate::xdg;

/// how often new audio is analyzed for the level meter
const METER_ANALYSIS: Duration = Duration::from_millis(16);
/// how often the level meter is redrawn
const METER_REFRESH: Duration = Duration::from_millis(50);

/// captures audio and passes the capture status to `show` until SIGINT or SIGTERM
pub fn audio(mut show: impl FnMut(&StatusSnapshot)) -> Result<(), AppError> {
    // must run before any thread is spawned, see `logging::init`
    logging::init(&xdg::process_env).report();
    let signals = block_signals()?;
    let settings = CaptureSettings {
        interval: METER_ANALYSIS,
        spectrum: SpectrumConfig::default(),
    };
    let capture = Capture::spawn(settings, None)?;
    watch(signals, capture, METER_REFRESH, |capture| {
        show(&capture.status());
    })
}

/// analyzes audio with the configured bars and tuning and passes bar heights
/// and the current gain to `show` at the configured framerate, until SIGINT
/// or SIGTERM
///
/// the motion runs here at the framerate, holding the latest spectrum between
/// audio updates, which may arrive far less often
pub fn spectrum(
    options: &RunOptions,
    mut show: impl FnMut(&[f32], &StatusSnapshot, f32),
) -> Result<(), AppError> {
    logging::init(&xdg::process_env).report();
    let loaded = load_config(options)?;
    let signals = block_signals()?;
    let frame_time = frame_time(loaded.config.visualizer.framerate);
    let spectrum = SpectrumConfig::from_config(&loaded.config);
    let dynamics = Dynamics::new(spectrum.bars, DynamicsConfig::from_config(&loaded.config));
    let settings = CaptureSettings {
        interval: frame_time,
        spectrum,
    };
    let capture = Capture::spawn(settings, None)?;
    let mut motion = Motion::new(Arc::clone(capture.frames()), dynamics, frame_time);
    // redraws even while silent, so the status line stays live
    watch(signals, capture, frame_time, |capture| {
        motion.advance(Instant::now());
        show(motion.heights(), &capture.status(), motion.gain());
    })
}

/// calls `on_tick` every `refresh` until SIGINT or SIGTERM, then stops `capture`
fn watch(
    signals: Signals,
    capture: Capture,
    refresh: Duration,
    mut on_tick: impl FnMut(&Capture),
) -> Result<(), AppError> {
    // the loop borrows `capture`, so it lives in its own scope
    {
        let mut event_loop: EventLoop<'_, bool> = EventLoop::try_new()?;
        let handle = event_loop.handle();
        handle
            .insert_source(signals, |_, _, running| *running = false)
            .map_err(|err| err.error)?;
        handle
            .insert_source(Timer::immediate(), |_, _, _| {
                on_tick(&capture);
                TimeoutAction::ToDuration(refresh)
            })
            .map_err(|err| err.error)?;

        let mut running = true;
        while running {
            event_loop.dispatch(None, &mut running)?;
        }
    }
    capture.stop();
    Ok(())
}
