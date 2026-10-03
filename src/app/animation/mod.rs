//! steps bar motion when surfaces are ready for a frame, capped at the
//! framerate, and sleeps until the capture thread wakes it once at rest

#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use tracing::debug;

use crate::activity::Presence;
use crate::audio::motion::Motion;
use crate::render::Frame;

/// how often step statistics are logged while animating
const STATS_PERIOD: Duration = Duration::from_secs(1);

/// bar motion paced for drawing
#[derive(Debug)]
pub struct Animation {
    motion: Motion,
    frame_time: Duration,
    slack: Duration,
    ticking: bool,
    next_due: Option<Instant>,
    generation: u64,
    time: Instant,
    stats: Stats,
}

impl Animation {
    pub fn new(motion: Motion, frame_time: Duration) -> Self {
        Self {
            motion,
            frame_time,
            slack: frame_time / 4,
            ticking: false,
            next_due: None,
            // surfaces draw once the first time they see any generation
            generation: 1,
            time: Instant::now(),
            stats: Stats::default(),
        }
    }

    pub fn motion_mut(&mut self) -> &mut Motion {
        &mut self.motion
    }

    /// swaps in motion, for a new bar count; the next frame starts from it
    pub fn set_motion(&mut self, motion: Motion) {
        self.motion = motion;
        self.ticking = false;
        self.next_due = None;
    }

    pub fn set_frame_time(&mut self, frame_time: Duration) {
        self.frame_time = frame_time;
        self.slack = frame_time / 4;
        self.motion.set_frame_time(frame_time);
    }

    /// peak sample level of the newest audio frame, 0 while silent
    pub fn level(&mut self) -> f32 {
        self.motion.level()
    }

    /// steps the motion when a frame is due and returns what to show
    pub fn frame(&mut self, now: Instant, presence: Presence) -> Frame<'_> {
        if presence.shown {
            self.pace(now, presence.restless());
        } else {
            self.stop();
            // no bars to move and no wake per frame: the capture thread pings
            // when the level crosses the activity threshold
            self.motion.pause();
        }
        Frame {
            heights: self.motion.heights(),
            generation: self.generation,
            time: self.time,
            animating: self.ticking,
        }
    }

    /// counts buffers committed for the debug statistics
    pub fn drawn(&mut self, surfaces: usize) {
        self.stats.draws += surfaces;
    }

    fn pace(&mut self, now: Instant, restless: bool) {
        if !self.ticking && (restless || !self.motion.rest()) {
            debug!("bars moving");
            self.ticking = true;
            self.next_due = None;
            self.stats.restart(now, self.motion.frames_seen());
        }
        if !self.ticking || self.next_due.is_some_and(|due| now + self.slack < due) {
            return;
        }
        self.motion.advance(now);
        self.generation += 1;
        self.time = now;
        // keep the cadence of the deadlines, but never schedule into the past
        let next = self.next_due.unwrap_or(now) + self.frame_time;
        self.next_due = Some(if next > now {
            next
        } else {
            now + self.frame_time
        });
        self.stats.step(now, self.motion.frames_seen());
        if !restless && self.motion.rest() {
            debug!("bars at rest, main loop idle");
            self.ticking = false;
        }
    }

    fn stop(&mut self) {
        if self.ticking {
            debug!("nothing shown, main loop idle");
            self.ticking = false;
        }
    }
}

/// steps, new spectra, and draws per period for the debug log
#[derive(Debug)]
struct Stats {
    since: Instant,
    steps: u32,
    draws: usize,
    first_frame: u64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            since: Instant::now(),
            steps: 0,
            draws: 0,
            first_frame: 0,
        }
    }
}

impl Stats {
    fn restart(&mut self, now: Instant, frames_seen: u64) {
        self.since = now;
        self.steps = 0;
        self.draws = 0;
        self.first_frame = frames_seen;
    }

    fn step(&mut self, now: Instant, frames_seen: u64) {
        self.steps += 1;
        let elapsed = now.saturating_duration_since(self.since);
        if elapsed >= STATS_PERIOD {
            debug!(
                "bars: {} steps, {} new spectra, {} draws in {:.2} s",
                self.steps,
                frames_seen.saturating_sub(self.first_frame),
                self.draws,
                elapsed.as_secs_f32()
            );
            self.restart(now, frames_seen);
        }
    }
}
