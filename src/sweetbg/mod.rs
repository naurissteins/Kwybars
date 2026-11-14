//! the bounded native sweetbg subscription, independent of the app and Wayland

mod connection;
#[cfg(test)]
mod tests;
mod wire;

pub(crate) use connection::Connection;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Output {
    pub name: String,
    pub path: PathBuf,
    pub fit: u32,
    pub generation: u32,
}

impl Output {
    pub fn supported(&self) -> bool {
        self.fit == 0 && self.path.is_absolute()
    }
}

pub(crate) enum Event {
    Snapshot {
        revision: u32,
        outputs: Vec<Output>,
    },
    Ack {
        revision: u32,
        name: String,
        accepted: bool,
    },
}
