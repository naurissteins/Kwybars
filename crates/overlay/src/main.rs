mod app;
mod ui;

use std::path::PathBuf;
use std::time::Duration;

use kwybars_common::cli;
use kwybars_common::config::DaemonConfig;
use kwybars_common::notify::notify_error_with_cooldown;

const GTK_RENDERER_ENV: &str = "GSK_RENDERER";
const DEFAULT_GTK_RENDERER: &str = "cairo";

fn main() {
    apply_default_gtk_renderer();

    let config_path = match resolve_cli_config_path() {
        Ok(path) => path,
        Err(exit_code) => std::process::exit(exit_code),
    };

    if let Err(err) = kwybars_common::logging::init_logging("overlay") {
        eprintln!("kwybars-overlay logging init failed: {err}");
    }

    if let Err(err) = app::run(config_path) {
        tracing::error!("kwybars-overlay failed: {err}");
        let defaults = DaemonConfig::default();
        notify_error_with_cooldown(
            "overlay.fatal",
            "Kwybars Overlay Error",
            &format!("{err}"),
            defaults.notify_on_error,
            Duration::from_secs(defaults.notify_cooldown_seconds),
        );
        std::process::exit(1);
    }
}

fn apply_default_gtk_renderer() {
    if std::env::var_os(GTK_RENDERER_ENV).is_some() {
        return;
    }

    if let Err(err) = gtk::glib::setenv(GTK_RENDERER_ENV, DEFAULT_GTK_RENDERER, false) {
        eprintln!("kwybars-overlay: failed to set default {GTK_RENDERER_ENV}: {err}");
    }
}

fn resolve_cli_config_path() -> Result<PathBuf, i32> {
    let options = match cli::parse_standard_cli() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("kwybars-overlay: {}", err.message());
            eprintln!("{}", cli::usage("kwybars-overlay"));
            return Err(2);
        }
    };

    if options.show_help {
        println!("{}", cli::usage("kwybars-overlay"));
        return Err(0);
    }

    Ok(options
        .config_path
        .unwrap_or_else(kwybars_common::config::default_config_path))
}
