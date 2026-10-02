//! `kwybars list-themes`

use std::ffi::OsString;

use super::check::ConfigFile;
use super::report::Report;
use crate::app::RunOptions;
use crate::config::{self, ThemeOrigin};

pub fn report(options: &RunOptions, env: &dyn Fn(&str) -> Option<OsString>) -> Report {
    let mut report = Report::default();
    let file = match ConfigFile::locate(options, env) {
        Ok(file) => file,
        Err(err) => {
            report.error(err);
            return report;
        }
    };
    report.line("available themes");
    for theme in config::themes(&file.path, env) {
        report.line(match theme.origin {
            ThemeOrigin::File(path) => format!("- {} ({})", theme.name, path.display()),
            ThemeOrigin::BuiltIn => format!("- {} (built-in)", theme.name),
        });
    }
    report
}
