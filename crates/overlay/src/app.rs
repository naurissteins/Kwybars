use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, UNIX_EPOCH};

use gtk::glib;
use gtk::prelude::*;
use kwybars_common::config::{self, AppConfig};
use kwybars_common::notify::notify_error_with_cooldown;
use tracing::{error, info, warn};

use crate::theme::{self, ThemePalette};

const APP_ID: &str = "io.kwybars.overlay";
const CONFIG_POLL_INTERVAL: Duration = Duration::from_millis(180);

#[derive(Debug)]
pub enum AppError {
    Config(config::ConfigLoadError),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(err) => write!(f, "could not load config: {err}"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Config(err) => Some(err),
        }
    }
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

#[derive(Clone)]
struct RunningOverlay {
    windows: Vec<gtk::ApplicationWindow>,
    runtime: RuntimeConfig,
    stream: Rc<kwybars_engine::live::LiveFrameStream>,
}

type OverlayState = Rc<std::cell::RefCell<Option<RunningOverlay>>>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConfigFilesStamp {
    config: ConfigStamp,
    colors: ConfigStamp,
    theme: ConfigStamp,
}

impl ConfigFilesStamp {
    fn read(config_path: &Path, theme_path: Option<&Path>) -> Self {
        let resolved_config_path =
            resolve_runtime_config_path(config_path).unwrap_or_else(|| config_path.to_path_buf());
        let colors_path = config::default_colors_path(&resolved_config_path);
        Self {
            config: ConfigStamp::read(config_path),
            colors: ConfigStamp::read(&colors_path),
            theme: theme_path.map(ConfigStamp::read).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RuntimeConfig {
    app_config: AppConfig,
    theme_palette: Option<ThemePalette>,
    theme_path: Option<PathBuf>,
    resolved_config_path: PathBuf,
    colors_path: PathBuf,
}

pub fn run(config_path: PathBuf) -> Result<(), AppError> {
    let runtime = load_runtime_config(&config_path).map_err(AppError::Config)?;
    info!("kwybars-overlay starting");
    info!("config path: {}", config_path.display());
    info!(
        "resolved config path: {}",
        runtime.resolved_config_path.display()
    );
    info!("colors path: {}", runtime.colors_path.display());
    if let Some(theme_path) = runtime.theme_path.as_ref() {
        info!("theme path: {}", theme_path.display());
    }

    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| {
        let state = Rc::new(std::cell::RefCell::new(None));
        apply_config(app, &state, runtime.clone());

        let app_weak = app.downgrade();
        let config_path_for_reload = config_path.clone();
        let state_for_reload = Rc::clone(&state);
        let mut last_stamp =
            ConfigFilesStamp::read(&config_path_for_reload, runtime.theme_path.as_deref());

        glib::timeout_add_local(CONFIG_POLL_INTERVAL, move || {
            let Some(app) = app_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };

            let current_theme_path = state_for_reload
                .borrow()
                .as_ref()
                .and_then(|running| running.runtime.theme_path.clone());
            let next_stamp =
                ConfigFilesStamp::read(&config_path_for_reload, current_theme_path.as_deref());
            if next_stamp == last_stamp {
                return glib::ControlFlow::Continue;
            }
            last_stamp = next_stamp;

            match load_runtime_config(&config_path_for_reload) {
                Ok(next_runtime) => {
                    let should_apply = state_for_reload
                        .borrow()
                        .as_ref()
                        .map(|running| running.runtime != next_runtime)
                        .unwrap_or(true);
                    if should_apply {
                        info!("kwybars: config/colors/theme changed, reloading overlay");
                        apply_config(&app, &state_for_reload, next_runtime);
                    }
                }
                Err(err) => {
                    warn!("kwybars: config reload failed (keeping current settings): {err}");
                    let (notify_enabled, notify_cooldown) = state_for_reload
                        .borrow()
                        .as_ref()
                        .map(|running| {
                            (
                                running.runtime.app_config.daemon.notify_on_error,
                                Duration::from_secs(
                                    running.runtime.app_config.daemon.notify_cooldown_seconds,
                                ),
                            )
                        })
                        .unwrap_or((true, Duration::from_secs(45)));
                    notify_error_with_cooldown(
                        "overlay.config_reload_failed",
                        "Kwybars Config Error",
                        &format!("Config reload failed: {err}"),
                        notify_enabled,
                        notify_cooldown,
                    );
                }
            }

            glib::ControlFlow::Continue
        });
    });

    let args = ["kwybars-overlay"];
    let _exit = app.run_with_args(&args);

    Ok(())
}

fn apply_config(app: &gtk::Application, state: &OverlayState, next_runtime: RuntimeConfig) {
    let next_stream = state
        .borrow()
        .as_ref()
        .filter(|running| {
            !audio_stream_config_changed(&running.runtime.app_config, &next_runtime.app_config)
        })
        .map(|running| Rc::clone(&running.stream))
        .unwrap_or_else(|| crate::ui::spawn_frame_stream(&next_runtime.app_config));
    let next_windows = crate::ui::build_overlay_windows(
        app,
        next_runtime.app_config.clone(),
        next_runtime.theme_palette.clone(),
        Rc::clone(&next_stream),
    );
    let previous = state.borrow_mut().replace(RunningOverlay {
        windows: next_windows,
        runtime: next_runtime,
        stream: next_stream,
    });

    if let Some(running) = previous {
        for window in running.windows {
            window.close();
        }
    }
}

fn audio_stream_config_changed(current: &AppConfig, next: &AppConfig) -> bool {
    current.visualizer.backend != next.visualizer.backend
        || current.visualizer.bars != next.visualizer.bars
        || current.visualizer.framerate != next.visualizer.framerate
        || current.visualizer.pipewire_attack != next.visualizer.pipewire_attack
        || current.visualizer.pipewire_decay != next.visualizer.pipewire_decay
        || current.visualizer.pipewire_gain != next.visualizer.pipewire_gain
        || current.visualizer.pipewire_curve != next.visualizer.pipewire_curve
        || current.visualizer.pipewire_neighbor_mix != next.visualizer.pipewire_neighbor_mix
}

fn load_runtime_config(config_path: &Path) -> Result<RuntimeConfig, config::ConfigLoadError> {
    let resolved_config_path =
        resolve_runtime_config_path(config_path).unwrap_or_else(|| config_path.to_path_buf());
    let colors_path = config::default_colors_path(&resolved_config_path);
    let mut config = config::load_or_default(&resolved_config_path)?;
    match config::load_color_overrides(&colors_path) {
        Ok(overrides) => config::apply_color_overrides(&mut config, overrides),
        Err(err) => {
            warn!("kwybars: colors override load failed (using config.toml colors): {err}");
            notify_error_with_cooldown(
                "overlay.colors_load_failed",
                "Kwybars Colors Error",
                &format!("Could not load colors override: {err}"),
                config.daemon.notify_on_error,
                Duration::from_secs(config.daemon.notify_cooldown_seconds),
            );
        }
    }

    let (theme_palette, theme_path) = load_theme_for_config(&config, &resolved_config_path);
    Ok(RuntimeConfig {
        app_config: config,
        theme_palette,
        theme_path,
        resolved_config_path,
        colors_path,
    })
}

fn resolve_runtime_config_path(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

fn load_theme_for_config(
    config: &AppConfig,
    config_path: &Path,
) -> (Option<ThemePalette>, Option<PathBuf>) {
    let Some(theme_name) = config.visualizer.theme.as_deref() else {
        return (None, None);
    };
    let trimmed_name = theme_name.trim();
    if trimmed_name.is_empty() {
        return (None, None);
    }

    let theme_path = theme::resolve_theme_path(config_path, trimmed_name);
    match theme::load_theme_palette(&theme_path, trimmed_name, config.visualizer.theme_opacity) {
        Ok(palette) => (Some(palette), Some(theme_path)),
        Err(err) => {
            error!("kwybars: theme load failed (using configured rgba colors): {err}");
            notify_error_with_cooldown(
                "overlay.theme_load_failed",
                "Kwybars Theme Error",
                &format!("Theme load failed: {err}"),
                config.daemon.notify_on_error,
                Duration::from_secs(config.daemon.notify_cooldown_seconds),
            );
            (None, Some(theme_path))
        }
    }
}
