use std::process::ExitCode;

mod app;
mod draw;

use tracing::{error, info};

fn main() -> ExitCode {
    if let Err(err) = kwybars_common::logging::init_logging("overlay-next") {
        eprintln!("kwybars-overlay-next: logging init failed: {err}");
    }

    info!("kwybars-overlay-next starting");
    match app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            error!("kwybars-overlay-next failed: {err}");
            eprintln!("kwybars-overlay-next failed: {err}");
            ExitCode::FAILURE
        }
    }
}
