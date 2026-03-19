use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ControlError {
    Usage(String),
    Io(std::io::Error),
    InvalidTarget(String),
    Report(String),
}

impl Display for ControlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(f, "{message}"),
            Self::Io(err) => write!(f, "{err}"),
            Self::InvalidTarget(message) => write!(f, "{message}"),
            Self::Report(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ControlError {}

impl From<std::io::Error> for ControlError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl ControlError {
    pub fn usage_like(err: impl Display) -> Self {
        Self::InvalidTarget(err.to_string())
    }
}
