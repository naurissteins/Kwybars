use crate::config::BackgroundConfig;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct Response {
    pub value: f64,
}

impl Response {
    pub fn step(
        &mut self,
        target: f64,
        elapsed: Duration,
        attack_ms: u32,
        release_ms: u32,
    ) -> bool {
        let seconds = f64::from(if target > self.value {
            attack_ms
        } else {
            release_ms
        })
        .max(1.0)
            / 1000.0;
        self.value += (target - self.value) * -(-elapsed.as_secs_f64() / seconds).exp_m1();
        if (target - self.value).abs() < 0.0001 {
            self.value = target;
        }
        self.value != target
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Pose {
    pub zoom: f64,
    pub offset: (f64, f64),
    pub animated: bool,
}

pub struct Motion {
    response: Response,
    mid: Response,
    pulse: f64,
    hold_until: Option<Instant>,
    last: Option<Instant>,
    started: Instant,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            response: Response::default(),
            mid: Response::default(),
            pulse: 0.0,
            hold_until: None,
            last: None,
            started: Instant::now(),
        }
    }
}

impl Motion {
    pub fn pose(&mut self, config: &BackgroundConfig, bass: f32, mid: f32, now: Instant) -> Pose {
        let target = if (config.zoom == 0.0 && config.shake == 0.0) || !bass.is_finite() {
            0.0
        } else {
            f64::from(((bass - config.threshold).max(0.0) * config.sensitivity).clamp(0.0, 1.0))
        };
        let elapsed = self
            .last
            .map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        let bass_moving = self
            .response
            .step(target, elapsed, config.attack_ms, config.release_ms);
        let amount = self.response.value;
        let mid_moving = self.step_mid(config, mid, elapsed, now);
        let mid_amount = self.mid.value;
        let (zoom_limit, shake_limit) = config.motion_limits();
        let shake_amount = (f64::from(config.shake) * amount
            + f64::from(config.mid.shake) * mid_amount)
            .min(f64::from(shake_limit));
        let zoom_amount = (f64::from(config.zoom) * amount
            + f64::from(config.mid.zoom) * mid_amount)
            .min(f64::from(zoom_limit));
        let animated = bass_moving || mid_moving || shake_amount > 0.0;
        self.last = animated.then_some(now);
        let phase =
            now.saturating_duration_since(self.started).as_secs_f64() * std::f64::consts::TAU;
        Pose {
            zoom: 1.0 + 2.0 * f64::from(shake_limit) + zoom_amount,
            offset: (
                (phase * 13.0).sin() * shake_amount,
                (phase * 17.0).sin() * shake_amount,
            ),
            animated,
        }
    }

    fn step_mid(
        &mut self,
        config: &BackgroundConfig,
        signal: f32,
        elapsed: Duration,
        now: Instant,
    ) -> bool {
        let config = &config.mid;
        if !config.enabled || (config.zoom == 0.0 && config.shake == 0.0) {
            self.mid.value = 0.0;
            self.pulse = 0.0;
            self.hold_until = None;
            return false;
        }
        let held = self.hold_until.map_or(Duration::ZERO, |until| {
            elapsed.saturating_sub(now.saturating_duration_since(until))
        });
        self.mid
            .step(self.pulse, held, config.attack_ms, config.release_ms);
        if self.hold_until.is_some_and(|until| now >= until) {
            self.pulse = 0.0;
            self.hold_until = None;
        }
        self.mid.step(
            0.0,
            elapsed.saturating_sub(held),
            config.attack_ms,
            config.release_ms,
        );
        let hit = if signal.is_finite() {
            f64::from(((signal - config.threshold).max(0.0) * config.sensitivity).clamp(0.0, 1.0))
        } else {
            0.0
        };
        if hit > 0.0 && (hit >= self.pulse || self.hold_until.is_none()) {
            self.pulse = hit;
            // Let the attack rise even if the next paint is after this audio frame.
            self.hold_until = Some(
                now + Duration::from_millis(u64::from(config.attack_ms).saturating_mul(3).max(40)),
            );
        }
        self.mid.value > 0.0 || self.hold_until.is_some()
    }
}
