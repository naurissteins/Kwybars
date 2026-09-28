use std::ffi::OsString;
use std::path::PathBuf;

use super::{Command, UsageError, parse};
use crate::app::RunOptions;

fn parse_args(args: &[&str]) -> Result<Command, UsageError> {
    parse(args.iter().map(OsString::from))
}

fn run_with_config(path: &str) -> Command {
    Command::Run(RunOptions {
        config_path: Some(PathBuf::from(path)),
    })
}

#[test]
fn no_arguments_runs_with_default_config() {
    assert_eq!(
        parse_args(&[]).ok(),
        Some(Command::Run(RunOptions::default()))
    );
}

#[test]
fn config_flag_forms_are_equivalent() {
    let expected = Some(run_with_config("/tmp/custom.toml"));
    assert_eq!(parse_args(&["--config", "/tmp/custom.toml"]).ok(), expected);
    assert_eq!(parse_args(&["--config=/tmp/custom.toml"]).ok(), expected);
    assert_eq!(parse_args(&["-c", "/tmp/custom.toml"]).ok(), expected);
}

#[test]
fn help_and_version_flags() {
    assert_eq!(parse_args(&["--help"]).ok(), Some(Command::Help));
    assert_eq!(parse_args(&["-h"]).ok(), Some(Command::Help));
    assert_eq!(parse_args(&["--version"]).ok(), Some(Command::Version));
    assert_eq!(parse_args(&["-V"]).ok(), Some(Command::Version));
}

#[test]
fn help_wins_over_other_flags() {
    assert_eq!(
        parse_args(&["--version", "--config", "x.toml", "--help"]).ok(),
        Some(Command::Help)
    );
}

#[test]
fn missing_config_value_is_an_error() {
    assert!(matches!(
        parse_args(&["--config"]),
        Err(UsageError::Args(_))
    ));
}

#[test]
fn unknown_flag_is_an_error() {
    assert!(matches!(
        parse_args(&["--verbose"]),
        Err(UsageError::Args(_))
    ));
}

#[test]
fn unknown_command_is_named() {
    match parse_args(&["frobnicate"]) {
        Err(UsageError::UnknownCommand(name)) => assert_eq!(name, "frobnicate"),
        other => panic!("expected unknown command, got {other:?}"),
    }
}
