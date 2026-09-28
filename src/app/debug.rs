//! `kwybars debug ...` runners

use std::time::{Duration, Instant};

use calloop::EventLoop;
use calloop::signals::{Signal, Signals};
use calloop::timer::{TimeoutAction, Timer};

use super::{AppError, RunOptions, load_config, logging};
use crate::audio::capture::{Capture, CaptureSettings, StatusSnapshot};
use crate::audio::dynamics::{Dynamics, DynamicsConfig};
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
    let settings = CaptureSettings {
        interval: METER_ANALYSIS,
        spectrum: SpectrumConfig::default(),
    };
    watch(settings, METER_REFRESH, |capture| show(&capture.status()))
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
    let framerate = loaded.config.visualizer.framerate.max(1);
    let frame_time = Duration::from_secs_f32(1.0 / framerate as f32);
    let spectrum = SpectrumConfig::from_config(&loaded.config);
    let mut dynamics = Dynamics::new(spectrum.bars, DynamicsConfig::from_config(&loaded.config));
    let mut latest = vec![0.0; spectrum.bars];
    let mut last_frame: Option<Instant> = None;
    let settings = CaptureSettings {
        interval: frame_time,
        spectrum,
    };
    watch(settings, frame_time, |capture| {
        let now = Instant::now();
        let dt = last_frame
            .map_or(frame_time, |last| now - last)
            .as_secs_f32();
        last_frame = Some(now);
        let frame = capture.frames().read_into(&mut latest);
        dynamics.update((!frame.silent).then_some(latest.as_slice()), dt);
        show(
            dynamics.heights(),
            &capture.status(),
            dynamics.effective_gain(),
        );
    })
}

/// runs capture until SIGINT or SIGTERM, calling `on_tick` every `refresh`
fn watch(
    settings: CaptureSettings,
    refresh: Duration,
    mut on_tick: impl FnMut(&Capture),
) -> Result<(), AppError> {
    // blocks these signals in this thread; the capture thread spawned next
    // inherits the mask, so the signalfd below is the only receiver
    let signals = Signals::new(&[Signal::SIGINT, Signal::SIGTERM])?;
    let capture = Capture::spawn(settings)?;

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
