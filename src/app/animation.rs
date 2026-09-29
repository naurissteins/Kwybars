//! steps bar motion when surfaces are ready for a frame, capped at the
//! framerate, and sleeps until the capture thread wakes it once at rest

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

    /// peak sample level of the newest audio frame, 0 while silent
    pub fn level(&mut self) -> f32 {
        self.motion.level()
    }

    /// steps the motion when a frame is due and returns what to show
    pub fn frame(&mut self, now: Instant, presence: Presence) -> Frame<'_> {
        if presence.shown {
            self.pace(now, presence.fading);
        } else {
            self.stop();
            // no bars to move, only the level of every new frame matters; a
            // frame that races the request is read on the next wake
            while !self.motion.wait() {}
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

    fn pace(&mut self, now: Instant, fading: bool) {
        if !self.ticking && (fading || !self.motion.rest()) {
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
        if !fading && self.motion.rest() {
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::Animation;
    use crate::activity::Presence;
    use crate::audio::dynamics::{Dynamics, DynamicsConfig};
    use crate::audio::frame::FrameSlot;
    use crate::audio::motion::Motion;

    const FRAME: Duration = Duration::from_micros(16_667);
    const SHOWN: Presence = Presence {
        shown: true,
        fading: false,
    };
    const FADING: Presence = Presence {
        shown: true,
        fading: true,
    };
    const HIDDEN: Presence = Presence {
        shown: false,
        fading: false,
    };

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
        assert!(!animation.frame(start, SHOWN).animating);
        frames.publish(&[0.5, 0.5], 0.5);
        let first = animation.frame(start, SHOWN);
        assert!(first.animating);
        assert_eq!(first.time, start);
        let generation = first.generation;
        // a callback 5 ms later is too early, one 14 ms later is close enough
        assert_eq!(
            animation
                .frame(start + Duration::from_millis(5), SHOWN)
                .generation,
            generation
        );
        assert_eq!(
            animation
                .frame(start + Duration::from_millis(14), SHOWN)
                .generation,
            generation + 1
        );
    }

    #[test]
    fn sixty_hertz_callbacks_give_every_frame_and_144_hertz_are_capped() {
        for (period_us, expected) in [(16_667_u64, 59..=61), (6_944, 58..=62)] {
            let (frames, mut animation) = animation();
            frames.publish(&[0.5, 0.5], 0.5);
            let start = Instant::now();
            let mut last = animation.frame(start, SHOWN).generation;
            let mut steps = 0;
            for tick in 1..=(1_000_000 / period_us) {
                let now = start + Duration::from_micros(tick * period_us);
                let generation = animation.frame(now, SHOWN).generation;
                steps += usize::from(generation != last);
                last = generation;
            }
            assert!(expected.contains(&steps), "{period_us} us: {steps} steps");
        }
    }

    #[test]
    fn stops_at_rest_after_silence() {
        let (frames, mut animation) = animation();
        frames.publish(&[1.0, 1.0], 0.5);
        let mut now = Instant::now();
        animation.frame(now, SHOWN);
        frames.publish_silence();
        for _ in 0..120 {
            now += FRAME;
            if !animation.frame(now, SHOWN).animating {
                assert!(
                    animation
                        .frame(now, SHOWN)
                        .heights
                        .iter()
                        .all(|h| *h < 1e-3)
                );
                // resting asked for a wake on the next frame
                assert!(frames.publish(&[0.5, 0.5], 0.5));
                return;
            }
        }
        panic!("never came to rest");
    }

    #[test]
    fn a_fade_keeps_resting_bars_ticking() {
        let (frames, mut animation) = animation();
        let start = Instant::now();
        let first = animation.frame(start, FADING);
        assert!(first.animating);
        let generation = first.generation;
        let later = animation.frame(start + FRAME, FADING);
        assert_eq!(later.generation, generation + 1);
        assert_eq!(later.time, start + FRAME);
        assert!(!animation.frame(start + FRAME * 2, SHOWN).animating);
        assert!(frames.publish(&[0.5, 0.5], 0.5));
    }

    #[test]
    fn hidden_bars_do_not_step_but_wait_for_audio() {
        let (frames, mut animation) = animation();
        frames.publish(&[0.5, 0.5], 0.5);
        let start = Instant::now();
        let generation = animation.frame(start, SHOWN).generation;
        let hidden = animation.frame(start + FRAME, HIDDEN);
        assert!(!hidden.animating);
        assert_eq!(hidden.generation, generation);
        // sound is still playing, yet the next frame wakes the loop
        assert!(frames.publish(&[0.5, 0.5], 0.5));
        assert_eq!(animation.level(), 0.5);
    }
}
