use super::{assert_close, parse_ok};
use crate::config::ActivityConfig;

const LEGACY_DAEMON: &str = r#"
[visualizer]
backend = "cava"
pipewire_gain = 1.5

[daemon]
enabled = true
poll_interval_ms = 50
activity_threshold = 0.045
activate_delay_ms = 120
deactivate_delay_ms = 1800
stop_on_silence = false
notify_on_error = true
notify_cooldown_seconds = 30
overlay_command = "cargo"
overlay_args = ["run", "-p", "kwybars-overlay"]
"#;

#[test]
fn legacy_daemon_config_loads_with_grouped_warnings() {
    let parsed = parse_ok(LEGACY_DAEMON);
    let activity = &parsed.config.activity;
    assert_close(activity.threshold, 0.045);
    assert_eq!(
        (activity.activate_delay_ms, activity.deactivate_delay_ms),
        (120, 1800)
    );
    assert_eq!(
        parsed.warnings,
        vec![
            "daemon.enabled, daemon.overlay_args, daemon.overlay_command, daemon.poll_interval_ms, daemon.stop_on_silence: removed (kwybars runs as a single process without a daemon), ignored",
            "daemon.notify_cooldown_seconds, daemon.notify_on_error: removed (desktop notifications were removed, errors are in the log and `kwybars doctor`), ignored",
            "visualizer.backend, visualizer.pipewire_gain: removed (audio is captured from PipeWire directly), ignored",
            "[daemon] is deprecated, move these keys to [activity]: daemon.activity_threshold -> activity.threshold, daemon.activate_delay_ms -> activity.activate_delay_ms, daemon.deactivate_delay_ms -> activity.deactivate_delay_ms",
        ]
    );
}

#[test]
fn activity_section_wins_over_daemon() {
    let parsed = parse_ok(
        "[activity]\nthreshold = 0.1\n[daemon]\nactivity_threshold = 0.2\ndeactivate_delay_ms = 900\n",
    );
    let activity = &parsed.config.activity;
    assert_close(activity.threshold, 0.1);
    assert_eq!(activity.deactivate_delay_ms, 900);
    assert_eq!(
        parsed.warnings,
        vec![
            "[daemon] is deprecated, move these keys to [activity]: daemon.deactivate_delay_ms -> activity.deactivate_delay_ms",
            "daemon.activity_threshold: ignored, [activity] sets it",
        ]
    );
}

#[test]
fn activity_defaults_and_clamping() {
    assert_eq!(parse_ok("").config.activity, ActivityConfig::default());
    let parsed = parse_ok("[activity]\nthreshold = 2.0\n");
    assert_close(parsed.config.activity.threshold, 1.0);
    assert_eq!(parsed.warnings.len(), 1);
}

#[test]
fn removed_keys_in_output_visualizer_are_named() {
    let parsed = parse_ok(
        "[[overlay.outputs]]\nmonitor = \"DP-1\"\n[overlay.outputs.visualizer]\nbackend = \"cava\"\n",
    );
    assert_eq!(
        parsed.warnings,
        vec![
            "overlay.outputs[0].visualizer.backend: removed (audio is captured from PipeWire directly), ignored"
        ]
    );
}

#[test]
fn unknown_daemon_keys_stay_unknown() {
    let parsed = parse_ok("[daemon]\ntypo = 1\n");
    assert_eq!(parsed.warnings, vec!["daemon.typo: unknown key, ignored"]);
}

#[test]
fn audio_section_defaults_and_bounds() {
    use crate::config::AudioConfig;
    assert_eq!(parse_ok("").config.audio, AudioConfig::default());

    let parsed = parse_ok(
        "[audio]\nsensitivity = 2.5\nauto_sensitivity = false\nlow_cutoff_hz = 30\nhigh_cutoff_hz = 16000\nsmoothing = 0.5\n",
    );
    let audio = &parsed.config.audio;
    assert_eq!(
        (
            audio.sensitivity,
            audio.auto_sensitivity,
            audio.low_cutoff_hz,
            audio.high_cutoff_hz,
            audio.smoothing
        ),
        (2.5, false, 30.0, 16_000.0, 0.5)
    );
    assert!(parsed.warnings.is_empty());

    let parsed =
        parse_ok("[audio]\nsmoothing = 1.5\nlow_cutoff_hz = 5000\nhigh_cutoff_hz = 6000\n");
    assert_close(parsed.config.audio.smoothing, 0.95);
    assert_close(parsed.config.audio.high_cutoff_hz, 10_000.0);
    assert_eq!(parsed.warnings.len(), 2, "{:?}", parsed.warnings);
}
