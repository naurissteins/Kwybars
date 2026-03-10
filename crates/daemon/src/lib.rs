mod activity;
mod process;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};

use activity::{ActivityState, ActivityTracker};
use kwybars_common::config::{self, DaemonConfig, VisualizerConfig};
use kwybars_common::notify::notify_error_with_cooldown;
use kwybars_engine::live::LiveFrameStream;
use process::OverlayProcess;
use tracing::{error, info, warn};

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
    let mut stream = LiveFrameStream::spawn(runtime.visualizer.clone());
    info!("audio source: {:?}", stream.source_kind());
    let mut stream_restart_grace_until: Option<Instant> = None;

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
        if next_config_stamp != config_stamp {
            config_stamp = next_config_stamp;
            match load_runtime_config(&config_path) {
                Ok(next_runtime) => {
                    if runtime != next_runtime {
                        info!("kwybars-daemon: config changed, reloading daemon settings");
                        if overlay_launch_changed(&runtime.daemon, &next_runtime.daemon) {
                            info!(
                                "kwybars-daemon: overlay launch settings changed, restarting overlay"
                            );
                            overlay.stop().map_err(DaemonError::Runtime)?;
                        }
                        if audio_probe_config_changed(&runtime.visualizer, &next_runtime.visualizer)
                        {
                            stream = LiveFrameStream::spawn(next_runtime.visualizer.clone());
                            info!("audio source: {:?}", stream.source_kind());
                            if activity.state() == ActivityState::Active {
                                stream_restart_grace_until = Some(
                                    now + Duration::from_millis(
                                        next_runtime.daemon.deactivate_delay_ms,
                                    ),
                                );
                            }
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

        let peak = stream.latest_frame().peak;
        let mut instantaneous_active = peak >= runtime.daemon.activity_threshold;
        if !instantaneous_active
            && activity.state() == ActivityState::Active
            && stream_restart_grace_until.is_some_and(|until| now < until)
        {
            instantaneous_active = true;
        }
        if stream_restart_grace_until.is_some_and(|until| now >= until) {
            stream_restart_grace_until = None;
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
                if let Err(err) = overlay.ensure_running(&runtime.daemon, &config_path, now) {
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

fn resolve_runtime_config_path(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

#[cfg(test)]
mod tests {
    use kwybars_common::config::{VisualizerBackend, VisualizerConfig};

    use super::audio_probe_config_changed;

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
}
