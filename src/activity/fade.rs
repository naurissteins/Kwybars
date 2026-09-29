//! opacity moving linearly towards shown or hidden

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Fade {
    fade_in: Duration,
    fade_out: Duration,
    visible: bool,
    from: f32,
    since: Option<Instant>,
}

impl Fade {
    pub fn new(fade_in: Duration, fade_out: Duration) -> Self {
        Self {
            fade_in,
            fade_out,
            visible: false,
            from: 0.0,
            since: None,
        }
    }

    /// whether the opacity is heading for 1
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// heads for opacity 1 when `visible`, else for 0, from where it is at `now`
    pub fn set_visible(&mut self, visible: bool, now: Instant) {
        if visible == self.visible {
            return;
        }
        self.from = self.opacity(now);
        self.since = Some(now);
        self.visible = visible;
    }

    /// opacity in `0.0..=1.0` at `now`
    pub fn opacity(&self, now: Instant) -> f32 {
        let target = if self.visible { 1.0 } else { 0.0 };
        let Some(since) = self.since else {
            return self.from;
        };
        let duration = if self.visible {
            self.fade_in
        } else {
            self.fade_out
        };
        if duration.is_zero() {
            return target;
        }
        let step = now.saturating_duration_since(since).as_secs_f32() / duration.as_secs_f32();
        if self.visible {
            (self.from + step).min(1.0)
        } else {
            (self.from - step).max(0.0)
        }
    }

    /// whether the opacity still changes after `now`
    pub fn is_fading(&self, now: Instant) -> bool {
        let opacity = self.opacity(now);
        if self.visible {
            opacity < 1.0
        } else {
            opacity > 0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::Fade;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn fades_in_after_start() {
        let start = Instant::now();
        let mut fade = Fade::new(ms(100), ms(200));
        assert_eq!(fade.opacity(start), 0.0);
        assert!(!fade.is_fading(start));

        fade.set_visible(true, start);
        let half = fade.opacity(start + ms(50));
        assert!(half > 0.45 && half < 0.55, "{half}");
        assert!(fade.is_fading(start + ms(50)));
        assert_eq!(fade.opacity(start + ms(100)), 1.0);
        assert!(!fade.is_fading(start + ms(100)));
    }

    #[test]
    fn fades_out_from_wherever_it_was() {
        let start = Instant::now();
        let mut fade = Fade::new(ms(100), ms(200));
        fade.set_visible(true, start);
        // turned around at 0.5, half of a full fade-out remains
        fade.set_visible(false, start + ms(50));
        let quarter = fade.opacity(start + ms(100));
        assert!(quarter > 0.2 && quarter < 0.3, "{quarter}");
        assert_eq!(fade.opacity(start + ms(150)), 0.0);
        assert!(!fade.is_fading(start + ms(150)));
    }

    #[test]
    fn repeating_the_target_does_not_restart_the_fade() {
        let start = Instant::now();
        let mut fade = Fade::new(ms(100), ms(100));
        fade.set_visible(true, start);
        fade.set_visible(true, start + ms(50));
        assert_eq!(fade.opacity(start + ms(100)), 1.0);
    }

    #[test]
    fn zero_durations_jump() {
        let start = Instant::now();
        let mut fade = Fade::new(Duration::ZERO, Duration::ZERO);
        fade.set_visible(true, start);
        assert_eq!(fade.opacity(start), 1.0);
        fade.set_visible(false, start);
        assert_eq!(fade.opacity(start), 0.0);
        assert!(!fade.is_fading(start));
    }
}
