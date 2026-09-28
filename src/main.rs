//! kwybars: an audio visualizer overlay for Wayland desktops

mod app;
mod cli;
mod config;
mod xdg;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::parse(std::env::args_os().skip(1)) {
        Ok(command) => cli::execute(command),
        Err(err) => cli::report_usage_error(&err),
    }
}
