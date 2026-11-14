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
    last: Option<Instant>,
    started: Instant,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            response: Response::default(),
            last: None,
            started: Instant::now(),
        }
    }
}

impl Motion {
    pub fn pose(&mut self, config: &BackgroundConfig, bass: f32, now: Instant) -> Pose {
        let target = if (config.zoom == 0.0 && config.shake == 0.0) || !bass.is_finite() {
            0.0
        } else {
            f64::from(((bass - config.threshold).max(0.0) * config.sensitivity).clamp(0.0, 1.0))
        };
        let elapsed = self
            .last
            .map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        let moving = self
            .response
            .step(target, elapsed, config.attack_ms, config.release_ms);
        let amount = self.response.value;
        let animated = moving || (config.shake > 0.0 && amount > 0.0);
        self.last = animated.then_some(now);
        let phase =
            now.saturating_duration_since(self.started).as_secs_f64() * std::f64::consts::TAU;
        let shake = f64::from(config.shake);
        Pose {
            zoom: 1.0 + 2.0 * shake + f64::from(config.zoom) * amount,
            offset: (
                (phase * 13.0).sin() * shake * amount,
                (phase * 17.0).sin() * shake * amount,
            ),
            animated,
        }
    }
}
