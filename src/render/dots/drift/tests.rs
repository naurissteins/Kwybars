use std::time::{Duration, Instant};

use super::{BOUNCE, Drift, GRAVITY, JUMP, JUMP_THRESHOLD, MAX_STEP};

fn path(fps: u32, seconds: u32, height: impl Fn(f32) -> f32) -> Vec<f32> {
    let mut drift = Drift::new(1);
    let start = Instant::now();
    (0..=fps * seconds)
        .map(|frame| {
            let t = Duration::from_secs(1) * frame / fps;
            drift.step(&[height(t.as_secs_f32())], start + t);
            drift.lift(0)
        })
        .collect()
}

fn legacy_path(ticks: u32, height: impl Fn(f32) -> f32) -> Vec<f32> {
    let (mut offset, mut velocity) = (0.0_f32, 0.0_f32);
    (0..ticks)
        .map(|tick| {
            let value = height(tick as f32 / 60.0);
            if value > JUMP_THRESHOLD {
                velocity += value * JUMP;
            }
            velocity -= GRAVITY;
            offset += velocity;
            if offset < 0.0 {
                (offset, velocity) = (0.0, 0.0);
            } else if offset > 1.0 {
                offset = 1.0;
                velocity = -velocity * BOUNCE;
            }
            offset
        })
        .collect()
}

/// a loud swell over 0.3 s, then quiet, then a softer one
fn swells(t: f32) -> f32 {
    let swell = |start: f32, level: f32| {
        let phase = (t - start) / 0.3;
        if (0.0..1.0).contains(&phase) {
            level * (phase * std::f32::consts::PI).sin()
        } else {
            0.0
        }
    };
    swell(0.0, 0.9) + swell(1.0, 0.5)
}

#[test]
fn matches_legacy_at_sixty_fps_one_tick_behind() {
    let ours = path(60, 2, swells);
    let legacy = legacy_path(120, |t| swells(t + 1.0 / 60.0));
    for (frame, lift) in ours.iter().enumerate().skip(2) {
        let expected = legacy[frame - 2];
        assert!(
            (lift - expected).abs() < 1e-3,
            "frame {frame}: {lift} vs legacy {expected}"
        );
    }
    assert!(ours.iter().any(|lift| *lift > 0.5));
}

#[test]
fn motion_does_not_depend_on_the_framerate() {
    let sixty = path(60, 2, swells);
    for fps in [120, 144, 240] {
        let other = path(fps, 2, swells);
        let per = fps as usize / 12;
        for step in 1..24 {
            let lift = other[step * per];
            let near = &sixty[step * 5 - 1..=step * 5 + 1];
            let low = near.iter().copied().fold(f32::MAX, f32::min);
            let high = near.iter().copied().fold(f32::MIN, f32::max);
            assert!(
                (low - 0.01..=high + 0.01).contains(&lift),
                "{fps} fps at {step}/12 s: {lift} outside {near:?}"
            );
        }
    }
}

#[test]
fn a_loud_bar_holds_its_dot_at_the_far_side() {
    let lifts = path(144, 1, |_| 1.0);
    assert!(lifts[30..].iter().all(|lift| *lift > 0.95), "{lifts:?}");
}

#[test]
fn quiet_bars_leave_the_dots_settled() {
    let mut drift = Drift::new(3);
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);
    for frame in 0..10 {
        drift.step(&[0.04, 0.0, 0.05], at(frame * 16));
        assert!(drift.settled());
    }
    drift.step(&[0.0, 0.5, 0.0], at(176));
    assert!(!drift.settled());
    let mut ms = 176;
    while !drift.settled() && ms < 5_000 {
        ms += 16;
        drift.step(&[0.0; 3], at(ms));
    }
    assert!(drift.settled(), "still drifting after {ms} ms");
    assert_eq!(drift.lift(1), 0.0);
}

#[test]
fn a_stall_counts_as_a_short_step() {
    let start = Instant::now();
    let rising = |gap: Duration| {
        let mut drift = Drift::new(1);
        for frame in 0..4 {
            drift.step(&[0.6], start + Duration::from_millis(frame * 16));
        }
        drift.step(&[0.6], start + Duration::from_millis(48) + gap);
        drift.lift(0)
    };
    assert_eq!(rising(Duration::from_secs(30)), rising(MAX_STEP));
    // a settled dot got no frames while quiet: the pause is one tick
    let mut settled = Drift::new(1);
    settled.step(&[0.0], start);
    settled.step(&[1.0], start + Duration::from_secs(30));
    settled.step(&[1.0], start + Duration::from_millis(30_016));
    assert!(settled.lift(0) < 0.1, "{}", settled.lift(0));
}
