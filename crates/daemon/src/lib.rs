mod activity;
mod display;
mod process;
#[cfg(test)]
mod tests;
use activity::{ActivityState, ActivityTracker};
use kwybars_common::config::{self, DaemonConfig, OverlayConfig, VisualizerConfig};
use kwybars_common::notify::notify_error_with_cooldown;
use kwybars_engine::ipc::FrameSocketServer;
use kwybars_engine::live::{LiveFrameStream, SourceKind};
use process::OverlayProcess;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};
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
    overlay: OverlayConfig,
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
    let mut scheduled_overlay_stop: Option<Instant> = None;

    let mut activity = ActivityTracker::new();
    let mut overlay = OverlayProcess::new();

    loop {
        thread::sleep(Duration::from_millis(
            runtime.daemon.poll_interval_ms.max(16),
        ));
        let now = Instant::now();

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
                            scheduled_overlay_stop = None;
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

        if let Some(exit_status) = overlay.poll_exit().map_err(DaemonError::Runtime)? {
            let notify_overlay_exit = should_notify_overlay_exit(
                activity.state(),
                scheduled_overlay_stop,
                &runtime.daemon,
            );
            if notify_overlay_exit {
                warn!("kwybars-daemon: overlay exited with status {exit_status}");
                notify_error_with_cooldown(
                    "daemon.overlay_exited",
                    "Kwybars Overlay Exited",
                    &format!("Overlay process exited: {exit_status}"),
                    runtime.daemon.notify_on_error,
                    notify_cooldown(&runtime.daemon),
                );
            } else {
                info!("kwybars-daemon: overlay exited during managed shutdown ({exit_status})");
                scheduled_overlay_stop = None;
            }
        }

        match activity.state() {
            ActivityState::Active => {
                scheduled_overlay_stop = None;
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
                if runtime.daemon.stop_on_silence && overlay.is_running() {
                    let fade_out = Duration::from_millis(runtime.overlay.fade_out_ms);
                    if fade_out.is_zero() {
                        scheduled_overlay_stop = None;
                        overlay.stop().map_err(DaemonError::Runtime)?;
                    } else {
                        let stop_at = scheduled_overlay_stop.get_or_insert_with(|| {
                            info!(
                                "kwybars-daemon: stopping overlay after {} ms fade-out",
                                runtime.overlay.fade_out_ms
                            );
                            now + fade_out
                        });
                        if now >= *stop_at {
                            scheduled_overlay_stop = None;
                            overlay.stop().map_err(DaemonError::Runtime)?;
                        }
                    }
                } else {
                    scheduled_overlay_stop = None;
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
        overlay: app_config.overlay,
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

fn should_notify_overlay_exit(
    activity_state: ActivityState,
    scheduled_overlay_stop: Option<Instant>,
    daemon: &DaemonConfig,
) -> bool {
    if !daemon.stop_on_silence {
        return true;
    }

    activity_state == ActivityState::Active && scheduled_overlay_stop.is_none()
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
