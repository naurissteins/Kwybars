use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, UNIX_EPOCH};

use gtk::glib;
use gtk::prelude::*;
use kwybars_common::config::{self, AppConfig};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone)]
struct RunningOverlay {
    window: gtk::ApplicationWindow,
    config: AppConfig,
}

type OverlayState = Rc<std::cell::RefCell<Option<RunningOverlay>>>;

pub fn run() -> Result<(), AppError> {
    let config_path = config::default_config_path();
    let app_config = config::load_or_default(&config_path).map_err(AppError::Config)?;
    println!("kwybars-overlay starting");
    println!("config path: {}", config_path.display());

    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| {
        let state = Rc::new(std::cell::RefCell::new(None));
        apply_config(app, &state, app_config.clone());

        let app_weak = app.downgrade();
        let config_path_for_reload = config_path.clone();
        let state_for_reload = Rc::clone(&state);
        let mut last_stamp = ConfigStamp::read(&config_path_for_reload);

        glib::timeout_add_local(CONFIG_POLL_INTERVAL, move || {
            let Some(app) = app_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };

            let next_stamp = ConfigStamp::read(&config_path_for_reload);
            if next_stamp == last_stamp {
                return glib::ControlFlow::Continue;
            }
            last_stamp = next_stamp;

            match config::load_or_default(&config_path_for_reload) {
                Ok(next_config) => {
                    let should_apply = state_for_reload
                        .borrow()
                        .as_ref()
                        .map(|running| running.config != next_config)
                        .unwrap_or(true);
                    if should_apply {
                        eprintln!("kwybars: config changed, reloading overlay");
                        apply_config(&app, &state_for_reload, next_config);
                    }
                }
                Err(err) => {
                    eprintln!("kwybars: config reload failed (keeping current settings): {err}");
                }
            }

            glib::ControlFlow::Continue
        });
    });

    let _exit = app.run();

    Ok(())
}

fn apply_config(app: &gtk::Application, state: &OverlayState, next_config: AppConfig) {
    let next_window = crate::ui::build_overlay_window(app, next_config.clone());
    let previous = state.borrow_mut().replace(RunningOverlay {
        window: next_window,
        config: next_config,
    });

    if let Some(running) = previous {
        running.window.close();
    }
}
