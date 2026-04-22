use std::process::ExitCode;

use tracing::info;

fn main() -> ExitCode {
    if let Err(err) = kwybars_common::logging::init_logging("overlay-next") {
        eprintln!("kwybars-overlay-next: logging init failed: {err}");
    }

    info!("kwybars-overlay-next starting");
    println!("kwybars-overlay-next starting");
    ExitCode::SUCCESS
}
