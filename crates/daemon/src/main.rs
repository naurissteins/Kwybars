use std::time::Duration;

use kwybars_common::config::DaemonConfig;
use kwybars_common::notify::notify_error_with_cooldown;

fn main() {
    if let Err(err) = kwybars_common::logging::init_logging("daemon") {
        eprintln!("kwybars-daemon logging init failed: {err}");
    }

    if let Err(err) = kwybars_daemon::run() {
        tracing::error!("kwybars-daemon failed: {err}");
        let defaults = DaemonConfig::default();
        notify_error_with_cooldown(
            "daemon.fatal",
            "Kwybars Daemon Error",
            &format!("{err}"),
            defaults.notify_on_error,
            Duration::from_secs(defaults.notify_cooldown_seconds),
        );
        std::process::exit(1);
    }
}
