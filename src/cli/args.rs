//! command-line parsing

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use super::Command;
use crate::app::RunOptions;

/// command line that could not be parsed
#[derive(Debug, thiserror::Error)]
pub enum UsageError {
    #[error(transparent)]
    Args(#[from] lexopt::Error),
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("{command} needs {what}")]
    Missing {
        command: &'static str,
        what: &'static str,
    },
    #[error("{0} does not belong to this command")]
    StrayOption(&'static str),
}

/// options only some commands take
#[derive(Default)]
struct Extra {
    active: Option<PathBuf>,
    overlay_dir: Option<PathBuf>,
}

impl Extra {
    /// fails when an option is left that the command did not take
    fn finish(self) -> Result<(), UsageError> {
        match (self.active, self.overlay_dir) {
            (Some(_), _) => Err(UsageError::StrayOption("--active")),
            (_, Some(_)) => Err(UsageError::StrayOption("--overlay-dir")),
            (None, None) => Ok(()),
        }
    }
}

/// parses the arguments that follow the program name
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    use lexopt::prelude::*;

    let mut parser = lexopt::Parser::from_args(args);
    let mut options = RunOptions::default();
    let mut extra = Extra::default();
    let mut help = false;
    let mut version = false;
    let mut words: Vec<OsString> = Vec::new();

    while let Some(arg) = parser.next()? {
        match arg {
            Short('c') | Long("config") => {
                options.config_path = Some(PathBuf::from(parser.value()?));
            }
            Short('a') | Long("active") => extra.active = Some(PathBuf::from(parser.value()?)),
            Long("overlay-dir") => extra.overlay_dir = Some(PathBuf::from(parser.value()?)),
            Short('h') | Long("help") => help = true,
            Short('V') | Long("version") => version = true,
            Value(value) => words.push(value),
            _ => return Err(arg.unexpected().into()),
        }
    }

    if help {
        Ok(Command::Help)
    } else if version {
        Ok(Command::Version)
    } else {
        command(&words, options, extra)
    }
}

fn command(
    words: &[OsString],
    mut options: RunOptions,
    mut extra: Extra,
) -> Result<Command, UsageError> {
    let names: Vec<&str> = words
        .iter()
        .map(|word| word.to_str().unwrap_or_default())
        .collect();
    let command = match (names.as_slice(), words) {
        ([], _) => Command::Run(options),
        (["validate-config"], _) => Command::ValidateConfig(options),
        (["list-themes"], _) => Command::ListThemes(options),
        (["doctor"], _) => Command::Doctor(options),
        (["debug", "audio"], _) => Command::DebugAudio,
        (["debug", "spectrum"], _) => Command::DebugSpectrum(options),
        (["switch-config", rest @ ..], [_, paths @ ..]) if rest.len() <= 1 => {
            if let Some(active) = extra.active.take() {
                options.config_path = Some(active);
            }
            Command::SwitchConfig {
                options,
                target: one_path(paths, "switch-config", "the config file to switch to")?,
            }
        }
        (["image-overlay", "match", rest @ ..], [_, _, paths @ ..]) if rest.len() <= 1 => {
            const NAME: &str = "image-overlay match";
            Command::ImageOverlayMatch {
                options,
                overlay_dir: extra.overlay_dir.take().ok_or(UsageError::Missing {
                    command: NAME,
                    what: "--overlay-dir <DIR>",
                })?,
                wallpaper: one_path(paths, NAME, "the wallpaper path")?,
            }
        }
        _ => {
            let words: Vec<_> = words.iter().map(|word| word.to_string_lossy()).collect();
            return Err(UsageError::UnknownCommand(words.join(" ")));
        }
    };
    extra.finish()?;
    Ok(command)
}

fn one_path(
    paths: &[OsString],
    command: &'static str,
    what: &'static str,
) -> Result<PathBuf, UsageError> {
    paths
        .first()
        .map(OsStr::new)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or(UsageError::Missing { command, what })
}
