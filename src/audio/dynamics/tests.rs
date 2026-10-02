use super::{Dynamics, DynamicsConfig};

const DT: f32 = 1.0 / 60.0;

fn manual(smoothing: f32) -> DynamicsConfig {
    DynamicsConfig {
        sensitivity: 1.0,
        auto_sensitivity: false,
        smoothing,
    }
}

fn one_bar(config: DynamicsConfig) -> Dynamics {
    Dynamics::new(1, config)
}

fn run(dynamics: &mut Dynamics, raw: Option<&[f32]>, seconds: f32, dt: f32) -> f32 {
    let steps = (seconds / dt).round() as usize;
    for _ in 0..steps {
        dynamics.update(raw, dt);
    }
    dynamics.heights()[0]
}

#[test]
fn rises_immediately_without_smoothing() {
    let mut dynamics = one_bar(manual(0.0));
    assert_eq!(dynamics.update(Some(&[0.6]), DT), &[0.6]);
}

#[test]
fn falls_with_gravity_and_never_below_the_input() {
    let mut dynamics = one_bar(manual(0.0));
    dynamics.update(Some(&[1.0]), DT);
    let mut previous = 1.0;
    for _ in 0..20 {
        let height = dynamics.update(Some(&[0.2]), DT)[0];
        assert!(
            height <= previous && height >= 0.2,
            "{height} after {previous}"
        );
        previous = height;
    }
    // full height falls to the floor in about a third of a second
    assert!((run(&mut dynamics, Some(&[0.2]), 0.5, DT) - 0.2).abs() < 1e-6);
}

#[test]
fn silence_decays_to_rest_without_touching_the_gain() {
    let mut dynamics = one_bar(DynamicsConfig::default());
    run(&mut dynamics, Some(&[0.5]), 1.0, DT);
    let gain = dynamics.effective_gain();
    assert!(!dynamics.at_rest());
    run(&mut dynamics, None, 1.0, DT);
    assert!(dynamics.at_rest());
    assert_eq!(dynamics.effective_gain(), gain);
}

#[test]
fn auto_gain_settles_loud_input_just_below_the_top() {
    let mut dynamics = one_bar(DynamicsConfig::default());
    let height = run(&mut dynamics, Some(&[0.05]), 20.0, DT);
    assert!((0.8..=1.0).contains(&height), "{height}");

    let mut loud = one_bar(DynamicsConfig::default());
    run(&mut loud, Some(&[3.0]), 20.0, DT);
    assert!(loud.effective_gain() < 0.5, "{}", loud.effective_gain());
    assert!(loud.heights()[0] <= 1.0);
}

#[test]
fn manual_gain_is_the_sensitivity() {
    let config = DynamicsConfig {
        sensitivity: 2.0,
        ..manual(0.0)
    };
    let mut dynamics = one_bar(config);
    run(&mut dynamics, Some(&[0.1]), 5.0, DT);
    assert_eq!(dynamics.effective_gain(), 2.0);
    assert!((dynamics.heights()[0] - 0.2).abs() < 1e-6);
}

#[test]
fn behaves_the_same_at_different_update_rates() {
    // fixed gain, so both runs start from the same state; durations are whole
    // numbers of steps at both rates
    let config = manual(0.7);
    let mut fast = one_bar(config.clone());
    let mut slow = one_bar(config);
    let a = run(&mut fast, Some(&[0.9]), 0.5, 1.0 / 60.0);
    let b = run(&mut slow, Some(&[0.9]), 0.5, 1.0 / 24.0);
    assert!((a - b).abs() < 0.01, "rise: 60 Hz {a}, 24 Hz {b}");

    let a = run(&mut fast, None, 0.25, 1.0 / 60.0);
    let b = run(&mut slow, None, 0.25, 1.0 / 24.0);
    assert!((a - b).abs() < 0.01, "fall: 60 Hz {a}, 24 Hz {b}");
    assert!((a - (0.9 - 9.0 * 0.25 * 0.25)).abs() < 0.01, "gravity: {a}");
}

#[test]
fn non_finite_input_does_not_stick() {
    for smoothing in [0.0, 0.7] {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut dynamics = one_bar(manual(smoothing));
            run(&mut dynamics, Some(&[0.5]), 1.0, DT);
            let height = dynamics.update(Some(&[bad]), DT)[0];
            assert!((0.0..=1.0).contains(&height), "{bad}: {height}");
            let height = run(&mut dynamics, Some(&[0.5]), 2.0, DT);
            assert!((height - 0.5).abs() < 1e-3, "{bad}: {height}");
            run(&mut dynamics, None, 2.0, DT);
            assert!(dynamics.at_rest(), "{bad}");
        }
    }
}

#[test]
fn updating_does_not_reallocate() {
    let mut dynamics = Dynamics::new(3, DynamicsConfig::default());
    let before = (
        dynamics.heights.as_ptr(),
        dynamics.fall_from.as_ptr(),
        dynamics.fall_time.as_ptr(),
    );
    for round in 0..200 {
        let level = (round % 7) as f32 / 7.0;
        dynamics.update(Some(&[level, level, level]), DT);
        dynamics.update(None, DT);
    }
    let after = (
        dynamics.heights.as_ptr(),
        dynamics.fall_from.as_ptr(),
        dynamics.fall_time.as_ptr(),
    );
    assert_eq!(before, after);
}
