//! kwybars binary entrypoint

use std::process::ExitCode;

use kwybars::cli;

fn main() -> ExitCode {
    match cli::parse(std::env::args_os().skip(1)) {
        Ok(command) => cli::execute(command),
        Err(err) => cli::report_usage_error(&err),
    }
}
