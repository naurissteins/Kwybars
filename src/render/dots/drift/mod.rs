#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_nanos(1_000_000_000 / 60);
const GRAVITY: f32 = 0.042;
const JUMP: f32 = 0.10;
const JUMP_THRESHOLD: f32 = 0.05;
const BOUNCE: f32 = 0.2;
const MAX_STEP: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Dot {
    lift: f32,
    speed: f32,
    before: f32,
}

/// drift state of every dot of one surface
#[derive(Debug, Clone, PartialEq)]
pub struct Drift {
    dots: Vec<Dot>,
    carry: Duration,
    last: Option<Instant>,
}

impl Drift {
    pub fn new(dots: usize) -> Self {
        Self {
            dots: vec![Dot::default(); dots],
            carry: Duration::ZERO,
            last: None,
        }
    }

    pub fn step(&mut self, heights: &[f32], now: Instant) {
        let elapsed = self
            .last
            .map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        self.last = Some(now);
        // settled dots got no frames, so the time since the last one is idle
        let longest = if self.settled() { TICK } else { MAX_STEP };
        self.carry += elapsed.min(longest);
        while self.carry >= TICK {
            self.carry -= TICK;
            self.tick(heights);
        }
    }

    /// the legacy per-frame update
    fn tick(&mut self, heights: &[f32]) {
        for (index, dot) in self.dots.iter_mut().enumerate() {
            let height = heights.get(index).copied().unwrap_or(0.0);
            dot.before = dot.lift;
            if height > JUMP_THRESHOLD {
                dot.speed += height * JUMP;
            }
            dot.speed -= GRAVITY;
            dot.lift += dot.speed;
            if dot.lift < 0.0 {
                (dot.lift, dot.speed) = (0.0, 0.0);
            } else if dot.lift > 1.0 {
                dot.lift = 1.0;
                dot.speed = -dot.speed * BOUNCE;
            }
        }
    }

    /// how far dot `index` is from its edge, `0.0..=1.0`
    pub fn lift(&self, index: usize) -> f32 {
        let between = self.carry.as_secs_f32() / TICK.as_secs_f32();
        self.dots
            .get(index)
            .map_or(0.0, |dot| dot.before + (dot.lift - dot.before) * between)
    }

    pub fn settled(&self) -> bool {
        self.dots.iter().all(|dot| *dot == Dot::default())
    }
}
