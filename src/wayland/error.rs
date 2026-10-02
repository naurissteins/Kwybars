//! failures that stop the wayland side from starting

use smithay_client_toolkit::reexports::client::globals::{BindError, GlobalError};
use smithay_client_toolkit::reexports::client::{ConnectError, DispatchError};

/// the overlay cannot run on this compositor
#[derive(Debug, thiserror::Error)]
pub enum WaylandError {
    #[error("could not connect to a Wayland compositor: {0}")]
    Connect(#[from] ConnectError),
    #[error("could not read the compositor's globals: {0}")]
    Globals(#[from] GlobalError),
    #[error("the compositor does not provide {name}, which Kwybars needs ({source})")]
    Missing {
        name: &'static str,
        source: BindError,
    },
    #[error(
        "the compositor does not support wlr-layer-shell, which Kwybars needs to place its bars on the desktop"
    )]
    NoLayerShell,
    #[error("Wayland connection failed: {0}")]
    Dispatch(#[from] DispatchError),
    #[error("lost the connection to the compositor: {0}")]
    Lost(String),
}
