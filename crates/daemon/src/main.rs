fn main() {
    if let Err(err) = kwybars_common::logging::init_logging("daemon") {
        eprintln!("kwybars-daemon logging init failed: {err}");
    }

    if let Err(err) = kwybars_daemon::run() {
        tracing::error!("kwybars-daemon failed: {err}");
        std::process::exit(1);
    }
}
