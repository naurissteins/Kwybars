use std::error::Error;
use std::fmt::{Display, Formatter};
use std::time::Duration;

use smithay_client_toolkit::delegate_registry;
use smithay_client_toolkit::reexports::calloop::{self, EventLoop};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::globals::{
    GlobalError, GlobalList, registry_queue_init,
};
use smithay_client_toolkit::reexports::client::{ConnectError, Connection, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use tracing::info;

#[derive(Debug)]
pub enum AppError {
    Connect(ConnectError),
    RegistryInit(GlobalError),
    EventLoop(calloop::Error),
    InsertSource(calloop::InsertError<WaylandSource<AppState>>),
    Dispatch(calloop::Error),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(err) => write!(f, "failed to connect to Wayland compositor: {err}"),
            Self::RegistryInit(err) => write!(f, "failed to initialize Wayland registry: {err}"),
            Self::EventLoop(err) => write!(f, "failed to create calloop event loop: {err}"),
            Self::InsertSource(err) => {
                write!(f, "failed to attach Wayland source to event loop: {err}")
            }
            Self::Dispatch(err) => write!(f, "Wayland event loop dispatch failed: {err}"),
        }
    }
}

impl Error for AppError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AdvertisedGlobal {
    name: u32,
    interface: String,
    version: u32,
}

pub struct AppState {
    registry_state: RegistryState,
    initial_globals: Vec<AdvertisedGlobal>,
}

impl AppState {
    fn new(globals: &GlobalList) -> Self {
        let initial_globals = globals
            .contents()
            .clone_list()
            .into_iter()
            .map(|global| AdvertisedGlobal {
                name: global.name,
                interface: global.interface,
                version: global.version,
            })
            .collect();

        Self {
            registry_state: RegistryState::new(globals),
            initial_globals,
        }
    }

    fn log_initial_globals(&self) {
        if self.initial_globals.is_empty() {
            info!("no Wayland globals advertised");
            return;
        }

        info!("Wayland globals discovered: {}", self.initial_globals.len());
        for global in &self.initial_globals {
            info!(
                "global {} => {} v{}",
                global.name, global.interface, global.version
            );
        }
    }
}

delegate_registry!(AppState);

impl ProvidesRegistryState for AppState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    fn runtime_add_global(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        info!("runtime global added: {name} => {interface} v{version}");
    }

    fn runtime_remove_global(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        name: u32,
        interface: &str,
    ) {
        info!("runtime global removed: {name} => {interface}");
    }
}

pub fn run() -> Result<(), AppError> {
    let conn = Connection::connect_to_env().map_err(AppError::Connect)?;
    let (globals, event_queue) = registry_queue_init(&conn).map_err(AppError::RegistryInit)?;
    let mut state = AppState::new(&globals);

    info!("connected to Wayland compositor");
    state.log_initial_globals();

    let mut event_loop = EventLoop::<AppState>::try_new().map_err(AppError::EventLoop)?;
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn, event_queue)
        .insert(loop_handle)
        .map_err(AppError::InsertSource)?;

    // Bootstrap only: prove the Wayland source and calloop integration work together.
    event_loop
        .dispatch(Duration::ZERO, &mut state)
        .map_err(AppError::Dispatch)?;
    info!("Wayland event loop bootstrap completed");

    Ok(())
}
