//! bar motion on the main thread: follows the latest frame at the display rate

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::dynamics::{Dynamics, DynamicsConfig};
use super::frame::FrameSlot;

/// reads frames from the capture thread and moves the bars towards them
#[derive(Debug)]
pub struct Motion {
    frames: Arc<FrameSlot>,
    dynamics: Dynamics,
    latest: Vec<f32>,
    seen: u64,
    silent: bool,
    level: f32,
    last_advance: Option<Instant>,
    frame_time: Duration,
}

impl Motion {
    /// `frame_time` is the step used for the first advance after a rest
    pub fn new(frames: Arc<FrameSlot>, dynamics: Dynamics, frame_time: Duration) -> Self {
        Self {
            latest: vec![0.0; frames.len()],
            frames,
            dynamics,
            seen: 0,
            silent: true,
            level: 0.0,
            last_advance: None,
            frame_time,
        }
    }

    pub fn set_dynamics(&mut self, config: DynamicsConfig) {
        self.dynamics.set_config(config);
    }

    pub fn set_frame_time(&mut self, frame_time: Duration) {
        self.frame_time = frame_time;
    }

    /// reads the newest frame and moves the bars to where they are at now
    pub fn advance(&mut self, now: Instant) -> &[f32] {
        self.refresh();
        let dt = self
            .last_advance
            .map_or(self.frame_time, |last| now.saturating_duration_since(last));
        self.last_advance = Some(now);
        let target = (!self.silent).then_some(self.latest.as_slice());
        self.dynamics.update(target, dt.as_secs_f32())
    }

    /// whether sound is playing or the bars are still falling
    pub fn is_moving(&self) -> bool {
        !self.silent || !self.dynamics.at_rest()
    }

    pub fn rest(&mut self) -> bool {
        self.refresh();
        if self.is_moving() || !self.frames.request_wake(self.seen) {
            return false;
        }
        self.last_advance = None;
        true
    }

    pub fn wait(&mut self) -> bool {
        self.refresh();
        self.last_advance = None;
        self.frames.request_wake(self.seen)
    }

    /// peak sample level of the newest frame, 0 while silent
    pub fn level(&mut self) -> f32 {
        self.refresh();
        self.level
    }

    pub fn heights(&self) -> &[f32] {
        self.dynamics.heights()
    }

    /// gain applied to the spectrum, including sensitivity
    pub fn gain(&self) -> f32 {
        self.dynamics.effective_gain()
    }

    /// frames read from the capture thread so far
    pub fn frames_seen(&self) -> u64 {
        self.seen
    }

    fn refresh(&mut self) {
        if self.frames.frame_number() == self.seen {
            return;
        }
        let info = self.frames.read_into(&mut self.latest);
        self.seen = info.number;
        self.silent = info.silent;
        self.level = info.level;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::Motion;
    use crate::audio::dynamics::{Dynamics, DynamicsConfig};
    use crate::audio::frame::FrameSlot;

    const FRAME: Duration = Duration::from_micros(16_667);

    fn motion(bars: usize) -> (Arc<FrameSlot>, Motion) {
        let frames = Arc::new(FrameSlot::new(bars));
        let config = DynamicsConfig {
            sensitivity: 1.0,
            auto_sensitivity: false,
            smoothing: 0.0,
        };
        let motion = Motion::new(Arc::clone(&frames), Dynamics::new(bars, config), FRAME);
        (frames, motion)
    }

    #[test]
    fn follows_the_latest_frame_between_updates() {
        let (frames, mut motion) = motion(2);
        let start = Instant::now();
        frames.publish(&[0.5, 0.25], 0.5);
        assert_eq!(motion.advance(start), &[0.5, 0.25]);
        // no new audio: the last spectrum stays the target
        assert_eq!(motion.advance(start + FRAME), &[0.5, 0.25]);
        assert_eq!(motion.frames_seen(), 1);
    }

    #[test]
    fn rests_only_after_silence_and_the_fall() {
        let (frames, mut motion) = motion(1);
        let mut now = Instant::now();
        frames.publish(&[1.0], 0.5);
        motion.advance(now);
        assert!(!motion.rest());
        frames.publish_silence();
        let mut steps = 0;
        loop {
            now += FRAME;
            motion.advance(now);
            if motion.rest() {
                break;
            }
            steps += 1;
            assert!(steps < 60, "bars never came to rest");
        }
        assert_eq!(motion.heights(), &[0.0]);
        // the next frame is reported and restarts the motion
        assert!(frames.publish(&[0.3], 0.5));
        assert!(!motion.rest());
    }

    #[test]
    fn a_frame_that_arrived_before_resting_keeps_it_awake() {
        let (frames, mut motion) = motion(1);
        frames.publish_silence();
        motion.advance(Instant::now());
        frames.publish(&[0.4], 0.5);
        assert!(!motion.rest());
        assert!(!frames.publish(&[0.4], 0.5));
    }

    #[test]
    fn waiting_reports_the_level_and_asks_for_the_next_frame() {
        let (frames, mut motion) = motion(1);
        assert_eq!(motion.level(), 0.0);
        frames.publish(&[0.4], 0.2);
        assert_eq!(motion.level(), 0.2);
        // still moving towards the frame, but a waiting reader is woken anyway
        assert!(motion.wait());
        assert!(frames.publish(&[0.4], 0.3));
        frames.publish_silence();
        assert_eq!(motion.level(), 0.0);
        assert!(motion.wait());
        assert!(frames.publish(&[0.4], 0.3));
    }

    #[test]
    fn advancing_does_not_reallocate() {
        let (frames, mut motion) = motion(4);
        let before = (motion.latest.as_ptr(), motion.heights().as_ptr());
        let mut now = Instant::now();
        for round in 0..100 {
            frames.publish(&[round as f32 / 100.0; 4], 0.5);
            now += FRAME;
            motion.advance(now);
        }
        assert_eq!(before, (motion.latest.as_ptr(), motion.heights().as_ptr()));
    }
}
