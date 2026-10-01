//! `kwybars doctor`: the config and what the desktop offers Kwybars

mod audio;
mod desktop;
mod service;
#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::fmt::Debug;

use super::check::{self, ConfigFile};
use super::report::Report;
use crate::app::{self, RunOptions};
use crate::audio::capture;
use crate::config::Config;
use crate::wayland;

/// checks everything, connecting to the compositor and to pipewire
pub fn report(options: &RunOptions, env: &dyn Fn(&str) -> Option<OsString>) -> Report {
    let mut report = Report::default();
    report.line(format!("kwybars doctor ({})", env!("CARGO_PKG_VERSION")));

    let loaded = match ConfigFile::locate(options, env) {
        Ok(file) => check::config(&mut report, &file, env),
        Err(err) => {
            report.error(err);
            None
        }
    };
    let config = loaded.map(|loaded| loaded.config);
    if let Some(config) = &config {
        settings(&mut report, config);
    }

    desktop::session(&mut report, env);
    let probe = wayland::probe().map_err(|err| err.to_string());
    desktop::compositor(&mut report, probe.as_ref(), config.as_ref());
    audio::pipewire(&mut report, capture::probe().map_err(|err| err.to_string()));
    report.line(service::line(service::query().as_deref()));
    report.line(match app::log_file_path(env) {
        Some(path) => format!("log file: {}", path.display()),
        None => "log file: none, Kwybars logs to stderr only".to_owned(),
    });

    report.line(match report.errors() {
        0 => "summary: ok".to_owned(),
        errors => format!("summary: {errors} issue(s) found"),
    });
    report
}

fn settings(report: &mut Report, config: &Config) {
    let (overlay, visualizer) = (&config.overlay, &config.visualizer);
    report.line(format!(
        "overlay: position {}, layer {}",
        spelled(overlay.position),
        spelled(overlay.layer)
    ));
    report.line(format!(
        "visualizer: layout {}, {} bars, {} fps",
        spelled(visualizer.layout),
        visualizer.bars,
        visualizer.framerate
    ));
}

fn spelled(value: impl Debug) -> String {
    format!("{value:?}").to_lowercase()
}
