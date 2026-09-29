//! steps bar motion when surfaces are ready for a frame, capped at the
//! framerate, and sleeps until the capture thread wakes it once at rest

use std::time::{Duration, Instant};

use tracing::debug;

use crate::audio::motion::Motion;
use crate::render::Frame;

/// how often step statistics are logged while animating
const STATS_PERIOD: Duration = Duration::from_secs(1);

/// bar motion paced for drawing
#[derive(Debug)]
pub struct Animation {
    motion: Motion,
    frame_time: Duration,
    /// a frame this much early still counts, so callbacks that arrive a
    /// little before the deadline do not halve the framerate
    slack: Duration,
    active: bool,
    next_due: Option<Instant>,
    generation: u64,
    stats: Stats,
}

impl Animation {
    pub fn new(motion: Motion, frame_time: Duration) -> Self {
        Self {
            motion,
            frame_time,
            slack: frame_time / 4,
            active: false,
            next_due: None,
            // surfaces draw once the first time they see any generation
            generation: 1,
            stats: Stats::default(),
        }
    }

    /// starts animating when a frame is waiting, otherwise asks the capture
    /// thread for a ping on its next frame; called at startup and per ping
    pub fn wake(&mut self) {
        if self.active || self.motion.rest() {
            return;
        }
        debug!("sound: bars moving");
        self.active = true;
        self.next_due = None;
        self.stats
            .restart(Instant::now(), self.motion.frames_seen());
    }

    /// steps the motion when a frame is due and returns what to show
    pub fn frame(&mut self, now: Instant) -> Frame<'_> {
        if self.active && self.next_due.is_none_or(|due| now + self.slack >= due) {
            self.motion.advance(now);
            self.generation += 1;
            // keep the cadence of the deadlines, but never schedule into the past
            let next = self.next_due.unwrap_or(now) + self.frame_time;
            self.next_due = Some(if next > now {
                next
            } else {
                now + self.frame_time
            });
            self.stats.step(now, self.motion.frames_seen());
            if self.motion.rest() {
                debug!("silence: bars at rest, main loop idle");
                self.active = false;
            }
        }
        Frame {
            heights: self.motion.heights(),
            generation: self.generation,
            animating: self.active,
        }
    }

    /// counts buffers committed for the debug statistics
    pub fn drawn(&mut self, surfaces: usize) {
        self.stats.draws += surfaces;
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::Animation;
    use crate::audio::dynamics::{Dynamics, DynamicsConfig};
    use crate::audio::frame::FrameSlot;
    use crate::audio::motion::Motion;

    const FRAME: Duration = Duration::from_micros(16_667);

    fn animation() -> (Arc<FrameSlot>, Animation) {
        let frames = Arc::new(FrameSlot::new(2));
        let config = DynamicsConfig {
            sensitivity: 1.0,
            auto_sensitivity: false,
            smoothing: 0.0,
        };
        let motion = Motion::new(Arc::clone(&frames), Dynamics::new(2, config), FRAME);
        (frames, Animation::new(motion, FRAME))
    }

    #[test]
    fn steps_only_when_a_frame_is_due() {
        let (frames, mut animation) = animation();
        let start = Instant::now();
        assert!(!animation.frame(start).animating);
        frames.publish(&[0.5, 0.5]);
        animation.wake();
        let first = animation.frame(start);
        assert!(first.animating);
        let generation = first.generation;
        // a callback 5 ms later is too early, one 14 ms later is close enough
        assert_eq!(
            animation.frame(start + Duration::from_millis(5)).generation,
            generation
        );
        assert_eq!(
            animation
                .frame(start + Duration::from_millis(14))
                .generation,
            generation + 1
        );
    }

    #[test]
    fn sixty_hertz_callbacks_give_every_frame_and_144_hertz_are_capped() {
        for (period_us, expected) in [(16_667_u64, 59..=61), (6_944, 58..=62)] {
            let (frames, mut animation) = animation();
            frames.publish(&[0.5, 0.5]);
            animation.wake();
            let start = Instant::now();
            let mut last = animation.frame(start).generation;
            let mut steps = 0;
            for tick in 1..=(1_000_000 / period_us) {
                let now = start + Duration::from_micros(tick * period_us);
                let generation = animation.frame(now).generation;
                steps += usize::from(generation != last);
                last = generation;
            }
            assert!(expected.contains(&steps), "{period_us} us: {steps} steps");
        }
    }

    #[test]
    fn stops_at_rest_after_silence() {
        let (frames, mut animation) = animation();
        frames.publish(&[1.0, 1.0]);
        animation.wake();
        frames.publish_silence();
        let mut now = Instant::now();
        for _ in 0..120 {
            now += FRAME;
            if !animation.frame(now).animating {
                assert!(animation.frame(now).heights.iter().all(|h| *h < 1e-3));
                return;
            }
        }
        panic!("never came to rest");
    }
}
