use std::error::Error;
use std::fmt::{Display, Formatter};

use gtk::prelude::*;
use kwybars_common::config;

const APP_ID: &str = "io.kwybars.overlay";

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

pub fn run() -> Result<(), AppError> {
    let config_path = config::default_config_path();
    let app_config = config::load_or_default(&config_path).map_err(AppError::Config)?;
    println!("kwybars-overlay starting");
    println!("config path: {}", config_path.display());

    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| {
        crate::ui::build_overlay_window(app, app_config.clone());
    });

    let _exit = app.run();

    Ok(())
}
