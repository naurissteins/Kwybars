use kwybars_common::config::VisualizerConfig;
use kwybars_common::spectrum::SpectrumFrame;
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::delegate_compositor;
use smithay_client_toolkit::delegate_layer;
use smithay_client_toolkit::delegate_output;
use smithay_client_toolkit::delegate_registry;
use smithay_client_toolkit::delegate_shm;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::globals::GlobalList;
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_shm, wl_surface};
use smithay_client_toolkit::reexports::client::{Connection, Proxy, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::registry_handlers;
use smithay_client_toolkit::shell::{
    WaylandSurface,
    wlr_layer::{
        Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
        LayerSurfaceConfigure,
    },
};
use smithay_client_toolkit::shm::{Shm, ShmHandler, slot::SlotPool};
use tracing::{error, info};

use crate::draw;

use super::AppError;
use super::source::AppFrameSource;

const DEFAULT_WIDTH: u32 = 0;
const DEFAULT_HEIGHT: u32 = 96;
const FALLBACK_BUFFER_WIDTH: u32 = 512;
const SURFACE_NAMESPACE: &str = "kwybars-overlay-next";

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
    pool: SlotPool,
    width: u32,
    height: u32,
    frame_source: AppFrameSource,
    latest_frame: SpectrumFrame,
    configured: bool,
    exit: bool,
}

impl AppState {
    pub fn new(
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
        visualizer: VisualizerConfig,
    ) -> Result<Self, AppError> {
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
            Some(SURFACE_NAMESPACE),
            None,
        );
        layer_surface.set_anchor(Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM);
        layer_surface.set_size(DEFAULT_WIDTH, DEFAULT_HEIGHT);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer_surface.commit();

        let pool = SlotPool::new(
            (FALLBACK_BUFFER_WIDTH * DEFAULT_HEIGHT * 4) as usize,
            &shm_state,
        )
        .map_err(|err| AppError::BufferSetup(err.to_string()))?;
        let mut frame_source = AppFrameSource::from_visualizer_config(&visualizer);
        let latest_frame = frame_source.frame_at(0);

        Ok(Self {
            registry_state: RegistryState::new(globals),
            output_state: OutputState::new(globals, qh),
            initial_globals,
            compositor_state,
            shm_state,
            layer_shell,
            surface,
            layer_surface,
            pool,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            frame_source,
            latest_frame,
            configured: false,
            exit: false,
        })
    }

    pub fn should_exit(&self) -> bool {
        self.exit
    }

    pub fn frame_source_description(&self) -> String {
        self.frame_source.description()
    }

    pub fn log_initial_globals(&self) {
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

    pub fn log_bound_globals(&self) {
        let _ = &self.compositor_state;
        let _ = &self.layer_shell;
        let _ = &self.surface;
        let _ = &self.layer_surface;
        let _ = &self.pool;
        info!("bound required global: wl_compositor");
        info!(
            "bound required global: wl_shm ({} advertised formats so far)",
            self.shm_state.formats().len()
        );
        info!("bound required global: zwlr_layer_shell_v1");
    }

    fn current_dimensions(&self) -> (u32, u32) {
        let width = if self.width == 0 {
            FALLBACK_BUFFER_WIDTH
        } else {
            self.width
        };
        let height = if self.height == 0 {
            DEFAULT_HEIGHT
        } else {
            self.height
        };
        (width, height)
    }

    fn update_frame(&mut self, timestamp_millis: u64) {
        self.latest_frame = self.frame_source.frame_at(timestamp_millis);
    }

    fn draw_buffer(&mut self, qh: &QueueHandle<Self>) -> Result<(), AppError> {
        let (width, height) = self.current_dimensions();
        let stride = width as i32 * 4;

        let (buffer, canvas) = self
            .pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .map_err(|err| AppError::BufferSetup(err.to_string()))?;

        draw::render_bars(canvas, width, height, &self.latest_frame);

        self.layer_surface
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        self.layer_surface
            .wl_surface()
            .frame(qh, self.layer_surface.wl_surface().clone());
        buffer
            .attach_to(self.layer_surface.wl_surface())
            .map_err(|err| AppError::BufferSetup(err.to_string()))?;
        self.layer_surface.commit();
        Ok(())
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
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        time: u32,
    ) {
        self.update_frame(u64::from(time));
        if let Err(err) = self.draw_buffer(qh) {
            error!("kwybars-overlay-next failed to draw animated frame: {err}");
            self.exit = true;
        }
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
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let previous_dimensions = self.current_dimensions();
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

        let current_dimensions = self.current_dimensions();
        if !self.configured {
            info!(
                "layer surface configured: width={}, height={}",
                self.width, self.height
            );
        } else if current_dimensions != previous_dimensions {
            info!(
                "layer surface resized: width={}, height={}",
                self.width, self.height
            );
        }

        self.configured = true;
        self.update_frame(self.latest_frame.timestamp_millis);
        if let Err(err) = self.draw_buffer(qh) {
            error!("kwybars-overlay-next failed to draw fake bar buffer: {err}");
            self.exit = true;
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
