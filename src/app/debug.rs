//! `kwybars debug ...` runners

use std::time::Duration;

use calloop::EventLoop;
use calloop::signals::{Signal, Signals};
use calloop::timer::{TimeoutAction, Timer};

use super::{AppError, logging};
use crate::audio::capture::{Capture, StatusSnapshot};
use crate::xdg;

/// how often new audio is analyzed
const ANALYSIS_INTERVAL: Duration = Duration::from_millis(16);
/// how often the meter is redrawn
const REFRESH: Duration = Duration::from_millis(50);

/// captures audio and passes the capture status to `show` until SIGINT or SIGTERM
pub fn audio(mut show: impl FnMut(&StatusSnapshot)) -> Result<(), AppError> {
    // must run before any thread is spawned, see `logging::init`
    logging::init(&xdg::process_env).report();

    // blocks these signals in this thread; the capture thread spawned next
    // inherits the mask, so the signalfd below is the only receiver
    let signals = Signals::new(&[Signal::SIGINT, Signal::SIGTERM])?;
    let capture = Capture::spawn(ANALYSIS_INTERVAL)?;

    // the loop borrows `capture`, so it lives in its own scope
    {
        let mut event_loop: EventLoop<'_, bool> = EventLoop::try_new()?;
        let handle = event_loop.handle();
        handle
            .insert_source(signals, |_, _, running| *running = false)
            .map_err(|err| err.error)?;
        handle
            .insert_source(Timer::immediate(), |_, _, _| {
                show(&capture.status());
                TimeoutAction::ToDuration(REFRESH)
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
