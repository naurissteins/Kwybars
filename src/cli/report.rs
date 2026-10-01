//! the lines a checking subcommand prints and the exit code they add up to

use std::fmt::Display;
use std::process::ExitCode;

#[derive(Debug, Default)]
pub struct Report {
    lines: Vec<String>,
    errors: usize,
}

impl Report {
    pub fn line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    pub fn warning(&mut self, warning: impl Display) {
        self.lines.push(format!("warning: {warning}"));
    }

    /// a line that makes the command exit with 1
    pub fn error(&mut self, error: impl Display) {
        self.lines.push(format!("error: {error}"));
        self.errors += 1;
    }

    pub fn errors(&self) -> usize {
        self.errors
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// the process exit status: 1 with errors, else 0
    pub fn status(&self) -> u8 {
        u8::from(self.errors > 0)
    }

    /// prints the lines to stdout and returns the exit code
    pub fn print(&self) -> ExitCode {
        for line in self.lines() {
            println!("{line}");
        }
        ExitCode::from(self.status())
    }
}
