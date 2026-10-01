//! command-line parsing and dispatch

use std::path::PathBuf;
use std::process::ExitCode;

use crate::app::{self, RunOptions};
use crate::xdg;

mod args;
mod check;
mod doctor;
mod meter;
mod report;
mod rewrite;
mod spectrum;
#[cfg(test)]
mod tests;
mod themes;
mod validate;

pub use args::{UsageError, parse};

const USAGE: &str = "\
Usage: kwybars [OPTIONS] [COMMAND]

Runs the Kwybars audio visualizer overlay.

Commands:
  validate-config      Check the config, colors.toml, theme, and image overlay
  list-themes          List the themes the config can name
  doctor               Check the config and what Kwybars needs from the desktop
  switch-config <FILE> Make the config path a link to FILE
  image-overlay match --overlay-dir <DIR> <WALLPAPER>
                       Show the image in DIR that is named like WALLPAPER
  debug audio          Show a live level meter of the captured audio
  debug spectrum       Show the analyzed bars in the terminal

Options:
  -c, --config <PATH>  Use the config at PATH instead of the default location
  -a, --active <PATH>  With switch-config: the same as --config
      --overlay-dir <DIR>
                       With image-overlay match: where the overlay images are
  -h, --help           Print this help and exit
  -V, --version        Print the version and exit

The checking commands exit with 1 when they find an error, the other
commands when they fail.
";

/// what the command line asks Kwybars to do
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Run(RunOptions),
    Help,
    Version,
    ValidateConfig(RunOptions),
    ListThemes(RunOptions),
    Doctor(RunOptions),
    SwitchConfig {
        options: RunOptions,
        target: PathBuf,
    },
    ImageOverlayMatch {
        options: RunOptions,
        overlay_dir: PathBuf,
        wallpaper: PathBuf,
    },
    DebugAudio,
    DebugSpectrum(RunOptions),
}

/// runs a parsed command and returns the process exit code
pub fn execute(command: Command) -> ExitCode {
    match command {
        Command::Help => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Command::Version => {
            println!("kwybars {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Command::Run(options) => exit_code(app::run(options)),
        Command::ValidateConfig(options) => validate::report(&options, &xdg::process_env).print(),
        Command::ListThemes(options) => themes::report(&options, &xdg::process_env).print(),
        Command::Doctor(options) => doctor::report(&options, &xdg::process_env).print(),
        Command::SwitchConfig { options, target } => {
            done(rewrite::switch(&options, &target, &xdg::process_env))
        }
        Command::ImageOverlayMatch {
            options,
            overlay_dir,
            wallpaper,
        } => done(rewrite::match_image(
            &options,
            &overlay_dir,
            &wallpaper,
            &xdg::process_env,
        )),
        Command::DebugAudio => {
            let result = app::debug::audio(meter::show);
            meter::finish();
            exit_code(result)
        }
        Command::DebugSpectrum(options) => {
            let mut view = spectrum::View::default();
            let result = app::debug::spectrum(&options, |bars, status, gain| {
                view.show(bars, status, gain);
            });
            view.finish();
            exit_code(result)
        }
    }
}

fn exit_code(result: Result<(), app::AppError>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!("{err}");
            ExitCode::FAILURE
        }
    }
}

/// prints what a file-changing command did, or why it failed
fn done(result: Result<String, rewrite::RewriteError>) -> ExitCode {
    match result {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("kwybars: {err}");
            ExitCode::FAILURE
        }
    }
}

/// prints a usage error with the usage text and returns exit code 2
pub fn report_usage_error(err: &UsageError) -> ExitCode {
    eprintln!("kwybars: {err}\n\n{USAGE}");
    ExitCode::from(2)
}
