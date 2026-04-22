use std::error::Error;
use std::fmt::{Display, Formatter};
use std::time::Duration;

use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::delegate_compositor;
use smithay_client_toolkit::delegate_layer;
use smithay_client_toolkit::delegate_output;
use smithay_client_toolkit::delegate_registry;
use smithay_client_toolkit::delegate_shm;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::calloop::{self, EventLoop};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::globals::{
    GlobalError, GlobalList, registry_queue_init,
};
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_surface};
use smithay_client_toolkit::reexports::client::{ConnectError, Connection, Proxy, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::registry_handlers;
use smithay_client_toolkit::shell::{
    WaylandSurface,
    wlr_layer::{
        Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
        LayerSurfaceConfigure,
    },
};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use tracing::info;

const DEFAULT_WIDTH: u32 = 0;
const DEFAULT_HEIGHT: u32 = 96;
const DISPATCH_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub enum AppError {
    Connect(ConnectError),
    RegistryInit(GlobalError),
    BindGlobal { global: &'static str, err: String },
    EventLoop(calloop::Error),
    InsertSource(calloop::InsertError<WaylandSource<AppState>>),
    Dispatch(calloop::Error),
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(err) => write!(f, "failed to connect to Wayland compositor: {err}"),
            Self::RegistryInit(err) => write!(f, "failed to initialize Wayland registry: {err}"),
            Self::BindGlobal { global, err } => {
                write!(f, "failed to bind required global {global}: {err}")
            }
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
    output_state: OutputState,
    initial_globals: Vec<AdvertisedGlobal>,
    compositor_state: CompositorState,
    shm_state: Shm,
    layer_shell: LayerShell,
    surface: wl_surface::WlSurface,
    layer_surface: LayerSurface,
    width: u32,
    height: u32,
    configured: bool,
    exit: bool,
}

impl AppState {
    fn new(globals: &GlobalList, qh: &QueueHandle<Self>) -> Result<Self, AppError> {
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

        let compositor_state =
            CompositorState::bind(globals, qh).map_err(|err| AppError::BindGlobal {
                global: "wl_compositor",
                err: err.to_string(),
            })?;
        let shm_state = Shm::bind(globals, qh).map_err(|err| AppError::BindGlobal {
            global: "wl_shm",
            err: err.to_string(),
        })?;
        let layer_shell = LayerShell::bind(globals, qh).map_err(|err| AppError::BindGlobal {
            global: "zwlr_layer_shell_v1",
            err: err.to_string(),
        })?;
        let surface = compositor_state.create_surface(qh);
        let layer_surface = layer_shell.create_layer_surface(
            qh,
            surface.clone(),
            Layer::Bottom,
            Some("kwybars-overlay-next"),
            None,
        );
        layer_surface.set_anchor(Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM);
        layer_surface.set_size(DEFAULT_WIDTH, DEFAULT_HEIGHT);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer_surface.commit();

        Ok(Self {
            registry_state: RegistryState::new(globals),
            output_state: OutputState::new(globals, qh),
            initial_globals,
            compositor_state,
            shm_state,
            layer_shell,
            surface,
            layer_surface,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            configured: false,
            exit: false,
        })
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

    fn log_bound_globals(&self) {
        let _ = &self.compositor_state;
        let _ = &self.layer_shell;
        let _ = &self.surface;
        let _ = &self.layer_surface;
        info!("bound required global: wl_compositor");
        info!(
            "bound required global: wl_shm ({} advertised formats so far)",
            self.shm_state.formats().len()
        );
        info!("bound required global: zwlr_layer_shell_v1");
    }
}

delegate_compositor!(AppState);
delegate_layer!(AppState);
delegate_output!(AppState);
delegate_registry!(AppState);
delegate_shm!(AppState);

impl CompositorHandler for AppState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        info!("surface scale factor changed to {new_factor}");
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        new_transform: wl_output::Transform,
    ) {
        info!("surface transform changed to {:?}", new_transform);
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        time: u32,
    ) {
        info!("frame callback received at {time}");
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for AppState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        info!("layer surface closed by compositor");
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.width = if configure.new_size.0 == 0 {
            DEFAULT_WIDTH
        } else {
            configure.new_size.0
        };
        self.height = if configure.new_size.1 == 0 {
            DEFAULT_HEIGHT
        } else {
            configure.new_size.1
        };

        if !self.configured {
            info!(
                "layer surface configured: width={}, height={}",
                self.width, self.height
            );
            self.configured = true;
        }
    }
}

impl OutputHandler for AppState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        info!("new output advertised: {:?}", output.id());
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        info!("output updated: {:?}", output.id());
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        info!("output destroyed: {:?}", output.id());
    }
}

impl ProvidesRegistryState for AppState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

impl ShmHandler for AppState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

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

    while !state.exit {
        event_loop
            .dispatch(DISPATCH_TIMEOUT, &mut state)
            .map_err(AppError::Dispatch)?;
    }

    info!("Wayland event loop exiting");
    Ok(())
}
