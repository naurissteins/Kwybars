//! schedules bar motion on the main loop: ticks at the framerate while bars
//! move, then sleeps until the capture thread wakes it with a new frame

use std::time::{Duration, Instant};

use calloop::LoopHandle;
use calloop::ping::PingSource;
use calloop::timer::{TimeoutAction, Timer};
use tracing::{debug, warn};

use crate::audio::motion::Motion;

/// how often tick statistics are logged while animating
const STATS_PERIOD: Duration = Duration::from_secs(1);

/// main loop state for the bars
#[derive(Debug)]
pub struct Animation {
    motion: Motion,
    frame_time: Duration,
    ticking: bool,
    stats: Stats,
}

impl Animation {
    pub fn new(motion: Motion, frame_time: Duration) -> Self {
        Self {
            motion,
            frame_time,
            ticking: false,
            stats: Stats::default(),
        }
    }

    /// starts ticking when a frame is waiting, otherwise sleeps until the
    /// capture thread pings; called at startup and on every ping
    pub fn wake<D: AsMut<Self> + 'static>(&mut self, handle: &LoopHandle<'static, D>) {
        if self.ticking || self.motion.rest() {
            return;
        }
        let inserted = handle.insert_source(Timer::immediate(), |deadline, (), data| {
            data.as_mut().tick(deadline)
        });
        match inserted {
            Ok(_) => {
                debug!("sound: bars moving");
                self.ticking = true;
                self.stats
                    .restart(Instant::now(), self.motion.frames_seen());
            }
            Err(err) => warn!("could not start the bar animation: {}", err.error),
        }
    }

    /// one step at the framerate; drops the timer once everything is at rest
    fn tick(&mut self, deadline: Instant) -> TimeoutAction {
        let now = Instant::now();
        self.motion.advance(now);
        self.stats.tick(now, self.motion.frames_seen());
        if self.motion.rest() {
            debug!("silence: bars at rest, main loop idle");
            self.ticking = false;
            return TimeoutAction::Drop;
        }
        // keep the cadence of the deadlines, but never schedule into the past
        let next = deadline + self.frame_time;
        TimeoutAction::ToInstant(if next > now {
            next
        } else {
            now + self.frame_time
        })
    }
}

/// registers the ping the capture thread uses to wake the animation
pub fn insert_wake<D: AsMut<Animation> + 'static>(
    handle: &LoopHandle<'static, D>,
    wake: PingSource,
) -> Result<(), calloop::Error> {
    let timers = handle.clone();
    handle
        .insert_source(wake, move |(), (), data| data.as_mut().wake(&timers))
        .map_err(|err| err.error)?;
    Ok(())
}

/// counts ticks and new spectra for the debug log
#[derive(Debug)]
struct Stats {
    since: Instant,
    ticks: u32,
    first_frame: u64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            since: Instant::now(),
            ticks: 0,
            first_frame: 0,
        }
    }
}

impl Stats {
    fn restart(&mut self, now: Instant, frames_seen: u64) {
        self.since = now;
        self.ticks = 0;
        self.first_frame = frames_seen;
    }

    fn tick(&mut self, now: Instant, frames_seen: u64) {
        self.ticks += 1;
        let elapsed = now.saturating_duration_since(self.since);
        if elapsed >= STATS_PERIOD {
            debug!(
                "bars: {} ticks, {} new spectra in {:.2} s",
                self.ticks,
                frames_seen.saturating_sub(self.first_frame),
                elapsed.as_secs_f32()
            );
            self.restart(now, frames_seen);
        }
    }
}
