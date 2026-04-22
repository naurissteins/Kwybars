use std::path::PathBuf;
use std::process::ExitCode;

mod app;
mod draw;

use kwybars_common::cli;
use tracing::{error, info};

fn main() -> ExitCode {
    let config_path = match resolve_cli_config_path() {
        Ok(path) => path,
        Err(code) => return ExitCode::from(code as u8),
    };

    if let Err(err) = kwybars_common::logging::init_logging("overlay-next") {
        eprintln!("kwybars-overlay-next: logging init failed: {err}");
    }

    info!("kwybars-overlay-next starting");
    match app::run(config_path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            error!("kwybars-overlay-next failed: {err}");
            eprintln!("kwybars-overlay-next failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn resolve_cli_config_path() -> Result<PathBuf, i32> {
    let options = match cli::parse_standard_cli() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("kwybars-overlay-next: {}", err.message());
            eprintln!("{}", cli::usage("kwybars-overlay-next"));
            return Err(2);
        }
    };

    if options.show_help {
        println!("{}", cli::usage("kwybars-overlay-next"));
        return Err(0);
    }

    Ok(options
        .config_path
        .unwrap_or_else(kwybars_common::config::default_config_path))
}
