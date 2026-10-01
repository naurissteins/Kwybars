//! command-line parsing and dispatch

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::app::{self, RunOptions};
use crate::xdg;

mod check;
mod doctor;
mod meter;
mod report;
mod spectrum;
#[cfg(test)]
mod tests;
mod themes;
mod validate;

const USAGE: &str = "\
Usage: kwybars [OPTIONS] [COMMAND]

Runs the Kwybars audio visualizer overlay.

Commands:
  validate-config      Check the config, colors.toml, theme, and image overlay
  list-themes          List the themes the config can name
  doctor               Check the config and what Kwybars needs from the desktop
  debug audio          Show a live level meter of the captured audio
  debug spectrum       Show the analyzed bars in the terminal

Options:
  -c, --config <PATH>  Use the config at PATH instead of the default location
  -h, --help           Print this help and exit
  -V, --version        Print the version and exit

The checking commands exit with 1 when they find an error.
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
    DebugAudio,
    DebugSpectrum(RunOptions),
}

/// command line that could not be parsed
#[derive(Debug, thiserror::Error)]
pub enum UsageError {
    #[error(transparent)]
    Args(#[from] lexopt::Error),
    #[error("unknown command: {0}")]
    UnknownCommand(String),
}

/// parses the arguments that follow the program name
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    use lexopt::prelude::*;

    let mut parser = lexopt::Parser::from_args(args);
    let mut options = RunOptions::default();
    let mut help = false;
    let mut version = false;
    let mut words: Vec<String> = Vec::new();

    while let Some(arg) = parser.next()? {
        match arg {
            Short('c') | Long("config") => {
                options.config_path = Some(PathBuf::from(parser.value()?));
            }
            Short('h') | Long("help") => help = true,
            Short('V') | Long("version") => version = true,
            Value(value) => words.push(value.to_string_lossy().into_owned()),
            _ => return Err(arg.unexpected().into()),
        }
    }

    Ok(if help {
        Command::Help
    } else if version {
        Command::Version
    } else {
        match words
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            [] => Command::Run(options),
            ["validate-config"] => Command::ValidateConfig(options),
            ["list-themes"] => Command::ListThemes(options),
            ["doctor"] => Command::Doctor(options),
            ["debug", "audio"] => Command::DebugAudio,
            ["debug", "spectrum"] => Command::DebugSpectrum(options),
            _ => return Err(UsageError::UnknownCommand(words.join(" "))),
        }
    })
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

/// prints a usage error with the usage text and returns exit code 2
pub fn report_usage_error(err: &UsageError) -> ExitCode {
    eprintln!("kwybars: {err}\n\n{USAGE}");
    ExitCode::from(2)
}
