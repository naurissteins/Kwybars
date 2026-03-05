use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityState {
    Inactive,
    Active,
}

pub struct ActivityTracker {
    state: ActivityState,
    active_since: Option<Instant>,
    inactive_since: Option<Instant>,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self {
            state: ActivityState::Inactive,
            active_since: None,
            inactive_since: None,
        }
    }

    pub fn state(&self) -> ActivityState {
        self.state
    }

    pub fn update(
        &mut self,
        now: Instant,
        instantaneous_active: bool,
        activate_delay: Duration,
        deactivate_delay: Duration,
    ) -> bool {
        match self.state {
            ActivityState::Inactive => {
                if instantaneous_active {
                    match self.active_since {
                        Some(since) if now.duration_since(since) >= activate_delay => {
                            self.state = ActivityState::Active;
                            self.active_since = None;
                            self.inactive_since = None;
                            return true;
                        }
                        None => {
                            if activate_delay.is_zero() {
                                self.state = ActivityState::Active;
                                self.active_since = None;
                                self.inactive_since = None;
                                return true;
                            }
                            self.active_since = Some(now);
                        }
                        _ => {}
                    }
                } else {
                    self.active_since = None;
                }
            }
            ActivityState::Active => {
                if instantaneous_active {
                    self.inactive_since = None;
                } else {
                    match self.inactive_since {
                        Some(since) if now.duration_since(since) >= deactivate_delay => {
                            self.state = ActivityState::Inactive;
                            self.active_since = None;
                            self.inactive_since = None;
                            return true;
                        }
                        None => {
                            if deactivate_delay.is_zero() {
                                self.state = ActivityState::Inactive;
                                self.active_since = None;
                                self.inactive_since = None;
                                return true;
                            }
                            self.inactive_since = Some(now);
                        }
                        _ => {}
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{ActivityState, ActivityTracker};

    #[test]
    fn applies_activate_and_deactivate_delays() {
        let start = Instant::now();
        let mut tracker = ActivityTracker::new();

        let changed = tracker.update(
            start,
            true,
            Duration::from_millis(200),
            Duration::from_millis(300),
        );
        assert!(!changed);
        assert_eq!(tracker.state(), ActivityState::Inactive);

        let changed = tracker.update(
            start + Duration::from_millis(220),
            true,
            Duration::from_millis(200),
            Duration::from_millis(300),
        );
        assert!(changed);
        assert_eq!(tracker.state(), ActivityState::Active);

        let changed = tracker.update(
            start + Duration::from_millis(250),
            false,
            Duration::from_millis(200),
            Duration::from_millis(300),
        );
        assert!(!changed);
        assert_eq!(tracker.state(), ActivityState::Active);

        let changed = tracker.update(
            start + Duration::from_millis(560),
            false,
            Duration::from_millis(200),
            Duration::from_millis(300),
        );
        assert!(changed);
        assert_eq!(tracker.state(), ActivityState::Inactive);
    }
}
