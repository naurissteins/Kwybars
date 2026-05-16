use std::time::{Duration, Instant};

pub(super) struct FadeController {
    opacity: f64,
    visible: bool,
    fade_in: Duration,
    fade_out: Duration,
    threshold: f32,
    deactivate_delay: Duration,
    inactive_since: Option<Instant>,
    last_update: Instant,
}

impl FadeController {
    pub(super) fn new(
        now: Instant,
        fade_in: Duration,
        fade_out: Duration,
        threshold: f32,
        deactivate_delay: Duration,
    ) -> Self {
        Self {
            opacity: if fade_in.is_zero() { 1.0 } else { 0.0 },
            visible: true,
            fade_in,
            fade_out,
            threshold,
            deactivate_delay,
            inactive_since: None,
            last_update: now,
        }
    }

    pub(super) fn opacity(&self) -> f64 {
        self.opacity
    }

    pub(super) fn update(&mut self, now: Instant, peak: f32) -> f64 {
        self.update_visibility(now, peak);

        let target = if self.visible { 1.0 } else { 0.0 };
        if (self.opacity - target).abs() < f64::EPSILON {
            self.last_update = now;
            return self.opacity;
        }

        let elapsed = now.saturating_duration_since(self.last_update);
        self.last_update = now;
        let duration = if self.visible {
            self.fade_in
        } else {
            self.fade_out
        };

        if duration.is_zero() {
            self.opacity = target;
            return self.opacity;
        }

        let step = elapsed.as_secs_f64() / duration.as_secs_f64();
        if self.visible {
            self.opacity = (self.opacity + step).min(1.0);
        } else {
            self.opacity = (self.opacity - step).max(0.0);
        }

        self.opacity
    }

    fn update_visibility(&mut self, now: Instant, peak: f32) {
        if peak >= self.threshold {
            self.visible = true;
            self.inactive_since = None;
            return;
        }

        let Some(inactive_since) = self.inactive_since else {
            if self.deactivate_delay.is_zero() {
                self.visible = false;
            } else {
                self.inactive_since = Some(now);
            }
            return;
        };

        if now.duration_since(inactive_since) >= self.deactivate_delay {
            self.visible = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::FadeController;

    #[test]
    fn fades_in_after_start() {
        let start = Instant::now();
        let mut fade = FadeController::new(
            start,
            Duration::from_millis(100),
            Duration::from_millis(200),
            0.1,
            Duration::from_millis(300),
        );

        assert_eq!(fade.opacity(), 0.0);
        let opacity = fade.update(start + Duration::from_millis(50), 0.5);
        assert!(opacity > 0.45 && opacity < 0.55);
        assert_eq!(fade.update(start + Duration::from_millis(100), 0.5), 1.0);
    }

    #[test]
    fn waits_for_deactivate_delay_before_fading_out() {
        let start = Instant::now();
        let mut fade = FadeController::new(
            start,
            Duration::ZERO,
            Duration::from_millis(200),
            0.1,
            Duration::from_millis(300),
        );

        assert_eq!(fade.update(start + Duration::from_millis(100), 0.0), 1.0);
        assert_eq!(fade.update(start + Duration::from_millis(350), 0.0), 1.0);

        let opacity = fade.update(start + Duration::from_millis(400), 0.0);
        assert!(opacity < 1.0);
    }

    #[test]
    fn active_audio_cancels_pending_fade_out() {
        let start = Instant::now();
        let mut fade = FadeController::new(
            start,
            Duration::ZERO,
            Duration::from_millis(200),
            0.1,
            Duration::from_millis(300),
        );

        assert_eq!(fade.update(start + Duration::from_millis(100), 0.0), 1.0);
        assert_eq!(fade.update(start + Duration::from_millis(250), 0.5), 1.0);
        assert_eq!(fade.update(start + Duration::from_millis(450), 0.0), 1.0);
    }
}
