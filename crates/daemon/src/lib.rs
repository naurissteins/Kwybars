mod activity;
mod process;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};

use activity::{ActivityState, ActivityTracker};
use kwybars_common::config::{self, DaemonConfig, VisualizerConfig};
use kwybars_engine::live::LiveFrameStream;
use process::OverlayProcess;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct ConfigStamp {
    exists: bool,
    modified_millis: u128,
    len: u64,
}

impl ConfigStamp {
    fn read(path: &Path) -> Self {
        let Ok(metadata) = std::fs::metadata(path) else {
            return Self {
                exists: false,
                modified_millis: 0,
                len: 0,
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
        }
    }
}

pub fn run() -> Result<(), DaemonError> {
    let config_path = config::default_config_path();
    let mut runtime = load_runtime_config(&config_path).map_err(DaemonError::Config)?;
    if !runtime.daemon.enabled {
        println!("kwybars-daemon: disabled in config ([daemon].enabled=false), exiting");
        return Ok(());
    }

    println!("kwybars-daemon starting");
    println!("config path: {}", config_path.display());

    let mut config_stamp = ConfigStamp::read(&config_path);
    let mut stream = LiveFrameStream::spawn(runtime.visualizer.clone());
    println!("audio source: {:?}", stream.source_kind());

    let mut activity = ActivityTracker::new();
    let mut overlay = OverlayProcess::new();

    loop {
        thread::sleep(Duration::from_millis(
            runtime.daemon.poll_interval_ms.max(16),
        ));
        let now = Instant::now();

        if let Some(exit_status) = overlay.poll_exit().map_err(DaemonError::Runtime)? {
            eprintln!("kwybars-daemon: overlay exited with status {exit_status}");
        }

        let next_config_stamp = ConfigStamp::read(&config_path);
        if next_config_stamp != config_stamp {
            config_stamp = next_config_stamp;
            match load_runtime_config(&config_path) {
                Ok(next_runtime) => {
                    if runtime != next_runtime {
                        eprintln!("kwybars-daemon: config changed, reloading daemon settings");
                        if runtime.visualizer != next_runtime.visualizer {
                            stream = LiveFrameStream::spawn(next_runtime.visualizer.clone());
                            println!("audio source: {:?}", stream.source_kind());
                        }
                        if !next_runtime.daemon.enabled {
                            overlay.stop().map_err(DaemonError::Runtime)?;
                            println!(
                                "kwybars-daemon: disabled in config ([daemon].enabled=false), exiting"
                            );
                            return Ok(());
                        }
                        runtime = next_runtime;
                    }
                }
                Err(err) => {
                    eprintln!(
                        "kwybars-daemon: config reload failed (keeping current settings): {err}"
                    );
                }
            }
        }

        let peak = stream.latest_frame().peak;
        let instantaneous_active = peak >= runtime.daemon.activity_threshold;
        let state_changed = activity.update(
            now,
            instantaneous_active,
            Duration::from_millis(runtime.daemon.activate_delay_ms),
            Duration::from_millis(runtime.daemon.deactivate_delay_ms),
        );

        if state_changed {
            match activity.state() {
                ActivityState::Active => println!("kwybars-daemon: audio active"),
                ActivityState::Inactive => println!("kwybars-daemon: audio inactive"),
            }
        }

        match activity.state() {
            ActivityState::Active => {
                if let Err(err) = overlay.ensure_running(&runtime.daemon, now) {
                    eprintln!("kwybars-daemon: could not launch overlay: {err}");
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
    let app_config = config::load_or_default(config_path)?;
    Ok(RuntimeConfig {
        visualizer: app_config.visualizer,
        daemon: app_config.daemon,
    })
}
