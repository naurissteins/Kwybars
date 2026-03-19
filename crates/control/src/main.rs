mod args;
mod diagnostics;
mod error;
mod switch;

use args::{Command, parse_args, usage};
use diagnostics::{doctor, list_themes, validate_config};
use error::ControlError;
use kwybars_common::config;
use switch::{switch_config, validate_target};

fn main() {
    match run() {
        Ok(Some(message)) => println!("{message}"),
        Ok(None) => {}
        Err(ControlError::Usage(message)) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
        Err(ControlError::Report(message)) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("kwybarsctl: {err}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<Option<String>, ControlError> {
    match parse_args(std::env::args_os().skip(1))? {
        Command::Help => Ok(Some(usage())),
        Command::SwitchConfig { target, active } => {
            let active_path = active.unwrap_or_else(config::default_config_path);
            let target_path = validate_target(&target)?;
            let message = switch_config(&active_path, &target_path)?;
            Ok(Some(message))
        }
        Command::ValidateConfig { path } => {
            let path = path.unwrap_or_else(config::default_config_path);
            let message = validate_config(&path)?;
            Ok(Some(message))
        }
        Command::Doctor { path } => {
            let path = path.unwrap_or_else(config::default_config_path);
            let message = doctor(&path)?;
            Ok(Some(message))
        }
        Command::ListThemes { path } => {
            let path = path.unwrap_or_else(config::default_config_path);
            let message = list_themes(&path);
            Ok(Some(message))
        }
    }
}
