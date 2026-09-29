//! whether audio is playing, debounced by the activate and deactivate delays

use std::time::{Duration, Instant};

use crate::config::ActivityConfig;

#[derive(Debug, Clone)]
pub struct ActivityTracker {
    threshold: f32,
    activate_delay: Duration,
    deactivate_delay: Duration,
    active: bool,
    since: Option<Instant>,
}

impl ActivityTracker {
    /// starts inactive
    pub fn new(config: &ActivityConfig) -> Self {
        Self {
            threshold: config.threshold,
            activate_delay: Duration::from_millis(config.activate_delay_ms),
            deactivate_delay: Duration::from_millis(config.deactivate_delay_ms),
            active: false,
            since: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn update(&mut self, now: Instant, level: f32) -> bool {
        let loud = level >= self.threshold;
        if loud == self.active {
            self.since = None;
            return false;
        }
        let since = *self.since.get_or_insert(now);
        if now.saturating_duration_since(since) < self.delay() {
            return false;
        }
        self.active = loud;
        self.since = None;
        true
    }

    /// when the state flips if the level keeps disagreeing with it
    pub fn deadline(&self) -> Option<Instant> {
        self.since.map(|since| since + self.delay())
    }

    fn delay(&self) -> Duration {
        if self.active {
            self.deactivate_delay
        } else {
            self.activate_delay
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::ActivityTracker;
    use crate::config::ActivityConfig;

    fn tracker(activate_delay_ms: u64, deactivate_delay_ms: u64) -> ActivityTracker {
        ActivityTracker::new(&ActivityConfig {
            threshold: 0.1,
            activate_delay_ms,
            deactivate_delay_ms,
        })
    }

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn applies_activate_and_deactivate_delays() {
        let start = Instant::now();
        let mut tracker = tracker(200, 300);

        assert!(!tracker.update(start, 0.5));
        assert!(!tracker.is_active());
        assert_eq!(tracker.deadline(), Some(start + ms(200)));

        assert!(tracker.update(start + ms(220), 0.5));
        assert!(tracker.is_active());
        assert_eq!(tracker.deadline(), None);

        assert!(!tracker.update(start + ms(250), 0.0));
        assert!(tracker.is_active());
        assert_eq!(tracker.deadline(), Some(start + ms(550)));

        assert!(tracker.update(start + ms(560), 0.0));
        assert!(!tracker.is_active());
    }

    #[test]
    fn a_short_burst_does_not_activate() {
        let start = Instant::now();
        let mut tracker = tracker(200, 300);
        tracker.update(start, 0.5);
        tracker.update(start + ms(100), 0.0);
        assert_eq!(tracker.deadline(), None);
        // the delay starts over with the next loud audio
        assert!(!tracker.update(start + ms(250), 0.5));
        assert!(!tracker.update(start + ms(400), 0.5));
        assert!(tracker.update(start + ms(450), 0.5));
    }

    #[test]
    fn active_audio_cancels_a_pending_deactivation() {
        let start = Instant::now();
        let mut tracker = tracker(0, 300);
        assert!(tracker.update(start, 0.5));
        tracker.update(start + ms(100), 0.0);
        tracker.update(start + ms(250), 0.5);
        assert!(!tracker.update(start + ms(450), 0.0));
        assert!(tracker.is_active());
        assert!(tracker.update(start + ms(750), 0.0));
    }

    #[test]
    fn zero_delays_flip_at_once() {
        let start = Instant::now();
        let mut tracker = tracker(0, 0);
        assert!(tracker.update(start, 0.1));
        assert!(tracker.update(start, 0.05));
        assert_eq!(tracker.deadline(), None);
    }
}
