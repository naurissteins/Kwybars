mod activity;
mod process;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};

use activity::{ActivityState, ActivityTracker};
use kwybars_common::config::{self, DaemonConfig, VisualizerConfig};
use kwybars_common::notify::notify_error_with_cooldown;
use kwybars_engine::ipc::FrameSocketServer;
use kwybars_engine::live::{LiveFrameStream, SourceKind};
use process::OverlayProcess;
use tracing::{error, info, warn};

const CONFIG_RELOAD_DEBOUNCE: Duration = Duration::from_millis(260);

#[derive(Debug)]
pub enum DaemonError {
    Config(config::ConfigLoadError),
    Runtime(std::io::Error),
}

impl Display for DaemonError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(err) => write!(f, "failed to load config: {err}"),
            Self::Runtime(err) => write!(f, "runtime error: {err}"),
        }
    }
}

impl Error for DaemonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Config(err) => Some(err),
            Self::Runtime(err) => Some(err),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RuntimeConfig {
    visualizer: VisualizerConfig,
    daemon: DaemonConfig,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct ConfigStamp {
    exists: bool,
    modified_millis: u128,
    len: u64,
    resolved_path: Option<PathBuf>,
}

impl ConfigStamp {
    fn read(path: &Path) -> Self {
        let Ok(metadata) = std::fs::metadata(path) else {
            return Self {
                exists: false,
                modified_millis: 0,
                len: 0,
                resolved_path: None,
            };
        };

        let modified_millis = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_millis())
            .unwrap_or(0);

        Self {
            exists: true,
            modified_millis,
            len: metadata.len(),
            resolved_path: resolve_runtime_config_path(path),
        }
    }
}

#[derive(Clone, Debug)]
struct PendingConfigReload {
    stamp: ConfigStamp,
    ready_at: Instant,
}

pub fn run(config_path: PathBuf) -> Result<(), DaemonError> {
    let mut runtime = load_runtime_config(&config_path).map_err(DaemonError::Config)?;
    if !runtime.daemon.enabled {
        info!("kwybars-daemon: disabled in config ([daemon].enabled=false), exiting");
        return Ok(());
    }

    info!("kwybars-daemon starting");
    if config_path.exists() {
        info!("config path: {} (found)", config_path.display());
    } else {
        info!(
            "config path: {} (not found, using built-in defaults)",
            config_path.display()
        );
    }

    let mut config_stamp = ConfigStamp::read(&config_path);
    let mut pending_config_reload: Option<PendingConfigReload> = None;
    let stream = Arc::new(Mutex::new(LiveFrameStream::spawn(
        runtime.visualizer.clone(),
    )));
    info!("audio source: {:?}", stream_source_kind(&stream));
    let frame_server =
        match FrameSocketServer::spawn(Arc::clone(&stream), runtime.visualizer.framerate) {
            Ok(server) => {
                info!(
                    "kwybars-daemon: sharing frames at {}",
                    server.path().display()
                );
                Some(server)
            }
            Err(err) => {
                warn!("kwybars-daemon: could not start frame sharing socket: {err}");
                None
            }
        };
    let mut inactivity_grace_until: Option<Instant> = None;

    let mut activity = ActivityTracker::new();
    let mut overlay = OverlayProcess::new();

    loop {
        thread::sleep(Duration::from_millis(
            runtime.daemon.poll_interval_ms.max(16),
        ));
        let now = Instant::now();

        if let Some(exit_status) = overlay.poll_exit().map_err(DaemonError::Runtime)? {
            warn!("kwybars-daemon: overlay exited with status {exit_status}");
            notify_error_with_cooldown(
                "daemon.overlay_exited",
                "Kwybars Overlay Exited",
                &format!("Overlay process exited: {exit_status}"),
                runtime.daemon.notify_on_error,
                notify_cooldown(&runtime.daemon),
            );
        }

        let next_config_stamp = ConfigStamp::read(&config_path);
        if next_config_stamp == config_stamp {
            pending_config_reload = None;
        } else {
            match pending_config_reload.as_mut() {
                Some(pending) if pending.stamp != next_config_stamp => {
                    pending.stamp = next_config_stamp.clone();
                    pending.ready_at = now + CONFIG_RELOAD_DEBOUNCE;
                }
                Some(_) => {}
                None => {
                    pending_config_reload = Some(PendingConfigReload {
                        stamp: next_config_stamp.clone(),
                        ready_at: now + CONFIG_RELOAD_DEBOUNCE,
                    });
                }
            }
        }

        if pending_config_reload
            .as_ref()
            .is_some_and(|pending| now >= pending.ready_at)
        {
            let Some(pending) = pending_config_reload.take() else {
                continue;
            };
            config_stamp = pending.stamp;
            match load_runtime_config(&config_path) {
                Ok(next_runtime) => {
                    if runtime != next_runtime {
                        info!("kwybars-daemon: config changed, reloading daemon settings");
                        inactivity_grace_until = extend_inactivity_grace(
                            inactivity_grace_until,
                            activity.state(),
                            now,
                            config_switch_grace_duration(&runtime.daemon, &next_runtime.daemon),
                        );
                        if overlay_launch_changed(&runtime.daemon, &next_runtime.daemon) {
                            info!(
                                "kwybars-daemon: overlay launch settings changed, restarting overlay"
                            );
                            overlay.stop().map_err(DaemonError::Runtime)?;
                        }
                        if audio_probe_config_changed(&runtime.visualizer, &next_runtime.visualizer)
                        {
                            replace_stream(&stream, next_runtime.visualizer.clone());
                            if let Some(frame_server) = frame_server.as_ref() {
                                frame_server.set_framerate(next_runtime.visualizer.framerate);
                            }
                            info!("audio source: {:?}", stream_source_kind(&stream));
                        }
                        if !next_runtime.daemon.enabled {
                            overlay.stop().map_err(DaemonError::Runtime)?;
                            info!(
                                "kwybars-daemon: disabled in config ([daemon].enabled=false), exiting"
                            );
                            return Ok(());
                        }
                        runtime = next_runtime;
                    }
                }
                Err(err) => {
                    warn!("kwybars-daemon: config reload failed (keeping current settings): {err}");
                    notify_error_with_cooldown(
                        "daemon.config_reload_failed",
                        "Kwybars Config Error",
                        &format!("Config reload failed: {err}"),
                        runtime.daemon.notify_on_error,
                        notify_cooldown(&runtime.daemon),
                    );
                }
            }
        }

        let peak = latest_peak(&stream);
        let mut instantaneous_active = peak >= runtime.daemon.activity_threshold;
        if !instantaneous_active
            && activity.state() == ActivityState::Active
            && inactivity_grace_until.is_some_and(|until| now < until)
        {
            instantaneous_active = true;
        }
        if inactivity_grace_until.is_some_and(|until| now >= until) {
            inactivity_grace_until = None;
        }
        let state_changed = activity.update(
            now,
            instantaneous_active,
            Duration::from_millis(runtime.daemon.activate_delay_ms),
            Duration::from_millis(runtime.daemon.deactivate_delay_ms),
        );

        if state_changed {
            match activity.state() {
                ActivityState::Active => info!("kwybars-daemon: audio active"),
                ActivityState::Inactive => info!("kwybars-daemon: audio inactive"),
            }
        }

        match activity.state() {
            ActivityState::Active => {
                let frame_socket_path = frame_server.as_ref().map(FrameSocketServer::path);
                if let Err(err) =
                    overlay.ensure_running(&runtime.daemon, &config_path, frame_socket_path, now)
                {
                    error!("kwybars-daemon: could not launch overlay: {err}");
                    notify_error_with_cooldown(
                        "daemon.overlay_launch_failed",
                        "Kwybars Overlay Start Failed",
                        &format!("Could not launch overlay: {err}"),
                        runtime.daemon.notify_on_error,
                        notify_cooldown(&runtime.daemon),
                    );
                }
            }
            ActivityState::Inactive => {
                if runtime.daemon.stop_on_silence {
                    overlay.stop().map_err(DaemonError::Runtime)?;
                }
            }
        }
    }
}

fn load_runtime_config(config_path: &Path) -> Result<RuntimeConfig, config::ConfigLoadError> {
    let resolved_config_path =
        resolve_runtime_config_path(config_path).unwrap_or_else(|| config_path.to_path_buf());
    let app_config = config::load_or_default(&resolved_config_path)?;
    Ok(RuntimeConfig {
        visualizer: app_config.visualizer,
        daemon: app_config.daemon,
    })
}

fn stream_source_kind(stream: &Arc<Mutex<LiveFrameStream>>) -> SourceKind {
    stream
        .lock()
        .map(|stream| stream.source_kind())
        .unwrap_or(SourceKind::Dummy)
}

fn latest_peak(stream: &Arc<Mutex<LiveFrameStream>>) -> f32 {
    stream
        .lock()
        .map(|stream| stream.latest_frame().peak)
        .unwrap_or(0.0)
}

fn replace_stream(stream: &Arc<Mutex<LiveFrameStream>>, config: VisualizerConfig) {
    match stream.lock() {
        Ok(mut stream) => {
            *stream = LiveFrameStream::spawn(config);
        }
        Err(err) => {
            error!("kwybars-daemon: could not replace poisoned audio stream: {err}");
        }
    }
}

fn notify_cooldown(config: &DaemonConfig) -> Duration {
    Duration::from_secs(config.notify_cooldown_seconds)
}

fn overlay_launch_changed(current: &DaemonConfig, next: &DaemonConfig) -> bool {
    current.overlay_command != next.overlay_command || current.overlay_args != next.overlay_args
}

fn audio_probe_config_changed(current: &VisualizerConfig, next: &VisualizerConfig) -> bool {
    current.backend != next.backend
        || current.bars != next.bars
        || current.framerate != next.framerate
        || current.pipewire_attack != next.pipewire_attack
        || current.pipewire_decay != next.pipewire_decay
        || current.pipewire_gain != next.pipewire_gain
        || current.pipewire_curve != next.pipewire_curve
        || current.pipewire_neighbor_mix != next.pipewire_neighbor_mix
}

fn config_switch_grace_duration(current: &DaemonConfig, next: &DaemonConfig) -> Duration {
    let millis = current
        .deactivate_delay_ms
        .max(next.deactivate_delay_ms)
        .max(2500);
    Duration::from_millis(millis)
}

fn extend_inactivity_grace(
    current_until: Option<Instant>,
    activity_state: ActivityState,
    now: Instant,
    duration: Duration,
) -> Option<Instant> {
    if activity_state != ActivityState::Active {
        return current_until;
    }

    let next_until = now + duration;
    match current_until {
        Some(existing) if existing > next_until => Some(existing),
        _ => Some(next_until),
    }
}

fn resolve_runtime_config_path(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use kwybars_common::config::{DaemonConfig, VisualizerBackend, VisualizerConfig};

    use super::{
        ActivityState, CONFIG_RELOAD_DEBOUNCE, ConfigStamp, PendingConfigReload,
        audio_probe_config_changed, config_switch_grace_duration, extend_inactivity_grace,
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
}
