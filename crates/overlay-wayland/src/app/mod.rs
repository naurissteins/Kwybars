mod error;
mod monitor;
mod source;
mod state;
mod surface;

use std::path::PathBuf;
use std::time::Duration;

use kwybars_common::config;
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::Connection;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use tracing::info;

pub use error::AppError;
use state::AppState;

const DISPATCH_TIMEOUT: Duration = Duration::from_millis(250);

pub fn run(config_path: PathBuf) -> Result<(), AppError> {
    let config_exists = config_path.exists();
    let app_config = config::load_or_default(&config_path).map_err(AppError::Config)?;
    let conn = Connection::connect_to_env().map_err(AppError::Connect)?;
    let (globals, event_queue) = registry_queue_init(&conn).map_err(AppError::RegistryInit)?;
    let qh = event_queue.handle();
    let mut state = AppState::new(&globals, &qh, app_config)?;

    info!("connected to Wayland compositor");
    if config_exists {
        info!("config path: {} (found)", config_path.display());
    } else {
        info!(
            "config path: {} (not found, using built-in defaults)",
            config_path.display()
        );
    }
    info!("frame source: {}", state.frame_source_description());
    state.log_initial_globals();
    state.log_bound_globals();
    state.log_selected_outputs();
    if state.surface_count() > 0 {
        info!(
            "created {} layer-shell surface(s) from overlay config and committed initial empty state",
            state.surface_count()
        );
    } else if state.is_waiting_for_named_outputs() {
        info!("deferring layer-surface creation until output names are available");
    } else {
        info!("waiting for output advertisement before creating layer surfaces");
    }

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
