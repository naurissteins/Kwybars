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
    animated: false,
};
const FADING: Presence = Presence {
    shown: true,
    fading: true,
    animated: false,
};
const HIDDEN: Presence = Presence {
    shown: false,
    fading: false,
    animated: false,
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
fn a_self_animating_layout_ticks_only_while_shown() {
    const ANIMATED: Presence = Presence {
        shown: true,
        fading: false,
        animated: true,
    };
    let (frames, mut animation) = animation();
    let start = Instant::now();
    let mut generation = animation.frame(start, ANIMATED).generation;
    for step in 1..=10 {
        let frame = animation.frame(start + FRAME * step, ANIMATED);
        assert!(frame.animating);
        assert_eq!(frame.generation, generation + 1);
        generation = frame.generation;
    }
    // hidden, the same layout gets no frames and the loop waits for audio
    let hidden = animation.frame(start + FRAME * 11, HIDDEN);
    assert!(!hidden.animating);
    assert_eq!(hidden.generation, generation);
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
