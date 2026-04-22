mod error;
mod state;

use std::time::Duration;

use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::Connection;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use tracing::info;

pub use error::AppError;
use state::AppState;

const DISPATCH_TIMEOUT: Duration = Duration::from_millis(250);

pub fn run() -> Result<(), AppError> {
    let conn = Connection::connect_to_env().map_err(AppError::Connect)?;
    let (globals, event_queue) = registry_queue_init(&conn).map_err(AppError::RegistryInit)?;
    let qh = event_queue.handle();
    let mut state = AppState::new(&globals, &qh)?;

    info!("connected to Wayland compositor");
    state.log_initial_globals();
    state.log_bound_globals();
    info!("created bottom-anchored layer-shell surface and committed initial empty state");

    let mut event_loop = EventLoop::<AppState>::try_new().map_err(AppError::EventLoop)?;
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn, event_queue)
        .insert(loop_handle)
        .map_err(AppError::InsertSource)?;

    while !state.should_exit() {
        event_loop
            .dispatch(DISPATCH_TIMEOUT, &mut state)
            .map_err(AppError::Dispatch)?;
    }

    info!("Wayland event loop exiting");
    Ok(())
}
