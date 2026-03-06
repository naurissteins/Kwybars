mod app;
mod theme;
mod ui;

fn main() {
    if let Err(err) = kwybars_common::logging::init_logging("overlay") {
        eprintln!("kwybars-overlay logging init failed: {err}");
    }

    if let Err(err) = app::run() {
        tracing::error!("kwybars-overlay failed: {err}");
        std::process::exit(1);
    }
}
