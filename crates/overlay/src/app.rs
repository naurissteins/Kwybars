use std::error::Error;
use std::fmt::{Display, Formatter};

use kwybars_common::config;
use kwybars_engine::pipeline::{DummySineSource, FrameSource};

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
    let mut source = DummySineSource::new(app_config.visualizer.bars);
    let frame = source.next_frame();

    println!("kwybars-overlay bootstrap");
    println!("config path: {}", config_path.display());
    println!(
        "overlay position: {:?}, bars: {}, sample peak: {:.3}",
        app_config.overlay.position,
        frame.bar_count(),
        frame.peak
    );
    println!("next step: replace dummy source with PipeWire/libcava + GTK layer-shell renderer");

    Ok(())
}
