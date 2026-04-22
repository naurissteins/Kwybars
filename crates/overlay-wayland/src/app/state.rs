use kwybars_common::config::{AppConfig, OverlayConfig, OverlayPosition};
use kwybars_common::spectrum::SpectrumFrame;
use kwybars_common::theme::ThemePalette;
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::delegate_compositor;
use smithay_client_toolkit::delegate_layer;
use smithay_client_toolkit::delegate_output;
use smithay_client_toolkit::delegate_registry;
use smithay_client_toolkit::delegate_shm;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::globals::GlobalList;
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_surface};
use smithay_client_toolkit::reexports::client::{Connection, Proxy, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::registry_handlers;
use smithay_client_toolkit::shell::{
    WaylandSurface,
    wlr_layer::{
        KeyboardInteractivity, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
    },
};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use tracing::{error, info};

use crate::draw;
use crate::draw::BarPaint;

use super::AppError;
use super::buffer::SurfaceBuffers;
use super::monitor::{OutputSelection, select_outputs};
use super::source::AppFrameSource;
use super::surface::SurfaceConfig;

const SURFACE_NAMESPACE: &str = "kwybars-overlay-next";

#[derive(Debug, Clone, PartialEq, Eq)]
struct AdvertisedGlobal {
    name: u32,
    interface: String,
    version: u32,
}

struct SurfaceInstance {
    output: Option<wl_output::WlOutput>,
    wl_surface: wl_surface::WlSurface,
    layer_surface: LayerSurface,
    buffers: SurfaceBuffers,
    width: u32,
    height: u32,
    configured: bool,
}

pub struct AppState {
    registry_state: RegistryState,
    output_state: OutputState,
    initial_globals: Vec<AdvertisedGlobal>,
    compositor_state: CompositorState,
    shm_state: Shm,
    layer_shell: LayerShell,
    overlay_config: OverlayConfig,
    surface_config: SurfaceConfig,
    position: OverlayPosition,
    paint: BarPaint,
    frame_source: AppFrameSource,
    latest_frame: SpectrumFrame,
    surfaces: Vec<SurfaceInstance>,
    waiting_for_named_outputs: bool,
    last_frame_time: Option<u32>,
    exit: bool,
}

impl AppState {
    pub fn new(
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
        app_config: AppConfig,
        theme_palette: Option<ThemePalette>,
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
        let output_state = OutputState::new(globals, qh);

        let surface_config = SurfaceConfig::from_app_config(&app_config);
        let paint = BarPaint::from_visualizer(
            &app_config.visualizer,
            theme_palette.map(|palette| palette.colors),
        );

        let mut frame_source = AppFrameSource::from_visualizer_config(&app_config.visualizer);
        let latest_frame = frame_source.frame_at(0);

        let mut state = Self {
            registry_state: RegistryState::new(globals),
            output_state,
            initial_globals,
            compositor_state,
            shm_state,
            layer_shell,
            overlay_config: app_config.overlay.clone(),
            surface_config,
            position: app_config.overlay.position,
            paint,
            frame_source,
            latest_frame,
            surfaces: Vec::new(),
            waiting_for_named_outputs: false,
            last_frame_time: None,
            exit: false,
        };
        state.ensure_surfaces(qh)?;
        Ok(state)
    }

    fn create_surface_instance(
        qh: &QueueHandle<Self>,
        compositor_state: &CompositorState,
        layer_shell: &LayerShell,
        shm_state: &Shm,
        surface_config: &SurfaceConfig,
        output: Option<wl_output::WlOutput>,
    ) -> Result<SurfaceInstance, AppError> {
        let wl_surface = compositor_state.create_surface(qh);
        let layer_surface = layer_shell.create_layer_surface(
            qh,
            wl_surface.clone(),
            surface_config.layer,
            Some(SURFACE_NAMESPACE),
            output.as_ref(),
        );
        layer_surface.set_anchor(surface_config.anchor);
        layer_surface.set_margin(
            surface_config.margins.top,
            surface_config.margins.right,
            surface_config.margins.bottom,
            surface_config.margins.left,
        );
        layer_surface.set_size(
            surface_config.requested_width,
            surface_config.requested_height,
        );
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer_surface.commit();

        let buffers = SurfaceBuffers::new(
            surface_config.fallback_width,
            surface_config.fallback_height,
            shm_state,
        )?;

        Ok(SurfaceInstance {
            output,
            wl_surface,
            layer_surface,
            buffers,
            width: 0,
            height: 0,
            configured: false,
        })
    }

    pub fn should_exit(&self) -> bool {
        self.exit
    }

    pub fn frame_source_description(&self) -> String {
        self.frame_source.description()
    }

    pub fn surface_count(&self) -> usize {
        self.surfaces.len()
    }

    pub fn is_waiting_for_named_outputs(&self) -> bool {
        self.waiting_for_named_outputs
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
        let _ = &self.shm_state;
        info!("bound required global: wl_compositor");
        info!(
            "bound required global: wl_shm ({} advertised formats so far)",
            self.shm_state.formats().len()
        );
        info!("bound required global: zwlr_layer_shell_v1");
    }

    pub fn log_selected_outputs(&self) {
        if self.waiting_for_named_outputs {
            info!("waiting for output names before resolving target monitors");
            return;
        }
        if self.surfaces.is_empty() {
            info!("no target outputs resolved yet");
            return;
        }

        for (index, surface) in self.surfaces.iter().enumerate() {
            let label = self
                .output_label(surface.output.as_ref())
                .unwrap_or_else(|| "compositor-default".to_owned());
            info!("overlay surface {} target output: {}", index + 1, label);
        }
    }

    fn output_label(&self, output: Option<&wl_output::WlOutput>) -> Option<String> {
        let output = output?;
        let info = self.output_state.info(output)?;
        if let Some(name) = info.name
            && !name.is_empty()
        {
            return Some(name);
        }
        if !info.model.is_empty() || !info.make.is_empty() {
            return Some(format!("{} {}", info.make, info.model).trim().to_owned());
        }
        Some(format!("output-{}", info.id))
    }

    fn output_size_for(&self, output: Option<&wl_output::WlOutput>) -> Option<(i32, i32)> {
        let candidate = output
            .and_then(|selected| self.output_state.info(selected))
            .or_else(|| {
                self.output_state
                    .outputs()
                    .find_map(|known| self.output_state.info(&known))
            })?;

        if let Some(logical_size) = candidate.logical_size
            && logical_size.0 > 0
            && logical_size.1 > 0
        {
            return Some(logical_size);
        }

        candidate
            .modes
            .iter()
            .find(|mode| mode.current)
            .or_else(|| candidate.modes.iter().find(|mode| mode.preferred))
            .map(|mode| mode.dimensions)
            .filter(|(width, height)| *width > 0 && *height > 0)
    }

    fn ensure_surfaces(&mut self, qh: &QueueHandle<Self>) -> Result<(), AppError> {
        if !self.surfaces.is_empty() {
            return Ok(());
        }

        match select_outputs(&self.output_state, &self.overlay_config) {
            OutputSelection::PendingNames => {
                self.waiting_for_named_outputs = true;
                Ok(())
            }
            OutputSelection::Ready(outputs) => {
                self.waiting_for_named_outputs = false;
                if outputs.is_empty() {
                    return Ok(());
                }

                for output in outputs {
                    self.surfaces.push(Self::create_surface_instance(
                        qh,
                        &self.compositor_state,
                        &self.layer_shell,
                        &self.shm_state,
                        &self.surface_config,
                        Some(output),
                    )?);
                }
                self.log_selected_outputs();
                info!(
                    "created {} layer-shell surface(s) after output selection",
                    self.surface_count()
                );
                Ok(())
            }
        }
    }

    fn current_dimensions(&self, index: usize) -> (u32, u32) {
        let surface = &self.surfaces[index];
        self.surface_config.resolved_dimensions(
            surface.width,
            surface.height,
            self.output_size_for(surface.output.as_ref()),
        )
    }

    fn update_frame(&mut self, timestamp_millis: u64) {
        self.latest_frame = self.frame_source.frame_at(timestamp_millis);
    }

    fn draw_surface(&mut self, index: usize, qh: &QueueHandle<Self>) -> Result<(), AppError> {
        if !self.surfaces[index].configured {
            return Ok(());
        }

        let (width, height) = self.current_dimensions(index);
        let surface = &mut self.surfaces[index];
        let wl_surface = surface.layer_surface.wl_surface().clone();
        let attached = surface
            .buffers
            .render_and_attach(width, height, &wl_surface, |canvas| {
                draw::render_bars(
                    canvas,
                    width,
                    height,
                    &self.latest_frame,
                    &self.position,
                    &self.paint,
                )
            })?;

        if attached {
            wl_surface.damage_buffer(0, 0, width as i32, height as i32);
        }
        wl_surface.frame(qh, wl_surface.clone());
        surface.layer_surface.commit();
        Ok(())
    }

    fn draw_all_surfaces(&mut self, qh: &QueueHandle<Self>) -> Result<(), AppError> {
        for index in 0..self.surfaces.len() {
            self.draw_surface(index, qh)?;
        }
        Ok(())
    }

    fn frame_driver_surface(&self) -> Option<&wl_surface::WlSurface> {
        self.surfaces
            .iter()
            .find(|surface| surface.configured)
            .map(|surface| &surface.wl_surface)
    }

    fn surface_index_for_layer(&self, layer: &LayerSurface) -> Option<usize> {
        self.surfaces
            .iter()
            .position(|surface| surface.layer_surface == *layer)
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
        surface: &wl_surface::WlSurface,
        time: u32,
    ) {
        let Some(driver_surface) = self.frame_driver_surface() else {
            return;
        };
        if driver_surface != surface {
            return;
        }
        if self.last_frame_time == Some(time) {
            return;
        }
        self.last_frame_time = Some(time);

        self.update_frame(u64::from(time));
        if let Err(err) = self.draw_all_surfaces(qh) {
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
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        if let Some(index) = self.surface_index_for_layer(layer) {
            let label = self
                .output_label(self.surfaces[index].output.as_ref())
                .unwrap_or_else(|| "compositor-default".to_owned());
            info!("layer surface closed by compositor: {}", label);
            self.surfaces.remove(index);
        }

        if self.surfaces.is_empty() {
            self.exit = true;
        }
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let Some(index) = self.surface_index_for_layer(layer) else {
            return;
        };

        let previous_dimensions = self.current_dimensions(index);
        let output_label = self
            .output_label(self.surfaces[index].output.as_ref())
            .unwrap_or_else(|| "compositor-default".to_owned());

        {
            let surface = &mut self.surfaces[index];
            surface.width = configure.new_size.0;
            surface.height = configure.new_size.1;
        }

        let current_dimensions = self.current_dimensions(index);
        let surface = &mut self.surfaces[index];
        if !surface.configured {
            info!(
                "layer surface configured for {}: width={}, height={}",
                output_label, surface.width, surface.height
            );
        } else if current_dimensions != previous_dimensions {
            info!(
                "layer surface resized for {}: width={}, height={}",
                output_label, surface.width, surface.height
            );
        }

        surface.configured = true;
        self.update_frame(self.latest_frame.timestamp_millis);
        if let Err(err) = self.draw_all_surfaces(qh) {
            error!("kwybars-overlay-next failed to draw surface: {err}");
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
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        info!("new output advertised: {:?}", output.id());
        if let Err(err) = self.ensure_surfaces(qh) {
            error!("kwybars-overlay-next failed to initialize output surfaces: {err}");
            self.exit = true;
        }
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        info!("output updated: {:?}", output.id());
        if let Err(err) = self.ensure_surfaces(qh) {
            error!("kwybars-overlay-next failed to initialize output surfaces: {err}");
            self.exit = true;
        }
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
