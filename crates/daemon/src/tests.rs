use std::time::{Duration, Instant};

use kwybars_common::config::{DaemonConfig, VisualizerBackend, VisualizerConfig};

use super::{
    ActivityState, CONFIG_RELOAD_DEBOUNCE, ConfigStamp, PendingConfigReload,
    audio_probe_config_changed, config_switch_grace_duration, extend_inactivity_grace,
    should_notify_overlay_exit,
};

#[test]
fn ignores_purely_visual_visualizer_changes() {
    let current = VisualizerConfig::default();
    let mut next = current.clone();
    next.layout = kwybars_common::config::VisualizerLayout::Polygon;
    next.bar_width = 42;
    next.gap = 7;
    next.color_mode = kwybars_common::config::VisualizerColorMode::Solid;
    next.center_offset_x = 10.0;
    next.polygon_rotation = 45.0;

    assert!(!audio_probe_config_changed(&current, &next));
}

#[test]
fn detects_audio_probe_changes() {
    let current = VisualizerConfig::default();
    let mut next = current.clone();
    next.backend = VisualizerBackend::Pipewire;
    assert!(audio_probe_config_changed(&current, &next));

    let mut next = current.clone();
    next.bars += 8;
    assert!(audio_probe_config_changed(&current, &next));

    let mut next = current.clone();
    next.pipewire_gain += 0.1;
    assert!(audio_probe_config_changed(&current, &next));
}

#[test]
fn config_switch_grace_has_minimum_duration() {
    let current = DaemonConfig {
        deactivate_delay_ms: 1200,
        ..DaemonConfig::default()
    };
    let next = DaemonConfig {
        deactivate_delay_ms: 1800,
        ..DaemonConfig::default()
    };

    assert_eq!(
        config_switch_grace_duration(&current, &next),
        Duration::from_millis(2500)
    );
}

#[test]
fn extend_inactivity_grace_only_when_active() {
    let now = Instant::now();
    let duration = Duration::from_secs(3);

    assert_eq!(
        extend_inactivity_grace(None, ActivityState::Inactive, now, duration),
        None
    );

    let active_until = extend_inactivity_grace(None, ActivityState::Active, now, duration);
    assert!(active_until.is_some_and(|until| until >= now + duration));
}

#[test]
fn suppresses_overlay_exit_notifications_during_managed_shutdown() {
    let daemon = DaemonConfig::default();
    assert!(!should_notify_overlay_exit(
        ActivityState::Inactive,
        None,
        &daemon
    ));

    assert!(!should_notify_overlay_exit(
        ActivityState::Active,
        Some(Instant::now()),
        &daemon
    ));

    assert!(should_notify_overlay_exit(
        ActivityState::Active,
        None,
        &daemon
    ));
}

#[test]
fn reports_overlay_exits_when_daemon_does_not_manage_silence() {
    let daemon = DaemonConfig {
        stop_on_silence: false,
        ..DaemonConfig::default()
    };

    assert!(should_notify_overlay_exit(
        ActivityState::Inactive,
        Some(Instant::now()),
        &daemon
    ));
}

#[test]
fn debounce_keeps_latest_ready_at_when_stamp_changes() {
    let first = ConfigStamp {
        exists: true,
        modified_millis: 1,
        len: 10,
        resolved_path: None,
    };
    let second = ConfigStamp {
        exists: true,
        modified_millis: 2,
        len: 10,
        resolved_path: None,
    };
    let start = Instant::now();
    let mut pending = PendingConfigReload {
        stamp: first,
        ready_at: start + Duration::from_millis(100),
    };

    if pending.stamp != second {
        pending.stamp = second.clone();
        pending.ready_at = start + CONFIG_RELOAD_DEBOUNCE;
    }

    assert_eq!(pending.stamp, second);
    assert!(pending.ready_at >= start + CONFIG_RELOAD_DEBOUNCE);
}
