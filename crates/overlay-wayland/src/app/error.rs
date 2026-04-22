use std::error::Error;
use std::fmt::{Display, Formatter};

use kwybars_common::config::ConfigLoadError;
use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::ConnectError;
use smithay_client_toolkit::reexports::client::globals::GlobalError;

use super::state::AppState;

#[derive(Debug)]
pub enum AppError {
    Config(ConfigLoadError),
    Connect(ConnectError),
    RegistryInit(GlobalError),
    BindGlobal { global: &'static str, err: String },
    BufferSetup(String),
    EventLoop(calloop::Error),
    InsertSource(calloop::InsertError<WaylandSource<AppState>>),
    Dispatch(calloop::Error),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(err) => write!(f, "could not load config: {err}"),
            Self::Connect(err) => write!(f, "failed to connect to Wayland compositor: {err}"),
            Self::RegistryInit(err) => write!(f, "failed to initialize Wayland registry: {err}"),
            Self::BindGlobal { global, err } => {
                write!(f, "failed to bind required global {global}: {err}")
            }
            Self::BufferSetup(err) => write!(f, "failed to set up shm buffer: {err}"),
            Self::EventLoop(err) => write!(f, "failed to create calloop event loop: {err}"),
            Self::InsertSource(err) => {
                write!(f, "failed to attach Wayland source to event loop: {err}")
            }
            Self::Dispatch(err) => write!(f, "Wayland event loop dispatch failed: {err}"),
        }
    }
}

impl Error for AppError {}
