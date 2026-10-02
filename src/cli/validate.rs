//! `kwybars validate-config`

use std::ffi::OsString;

use super::check::{self, ConfigFile};
use super::report::Report;
use crate::app::RunOptions;

pub fn report(options: &RunOptions, env: &dyn Fn(&str) -> Option<OsString>) -> Report {
    let mut report = Report::default();
    match ConfigFile::locate(options, env) {
        Ok(file) => {
            check::config(&mut report, &file, env);
        }
        Err(err) => report.error(err),
    }
    report.line(match report.errors() {
        0 => "config validation passed".to_owned(),
        errors => format!("config validation failed: {errors} error(s)"),
    });
    report
}
