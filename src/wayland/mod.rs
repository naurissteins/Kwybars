//! wayland connection, outputs, and one layer surface per selected output

mod error;
mod handlers;
mod outputs;
mod placement;
mod probe;
mod scale;
mod selection;
mod surface;

use std::time::Instant;

use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use smithay_client_toolkit::reexports::calloop::{LoopHandle, RegistrationToken};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::backend::{self, ObjectId};
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::{wl_output::WlOutput, wl_shm};
use smithay_client_toolkit::reexports::client::{
    Connection, DispatchError, EventQueue, QueueHandle,
};
use smithay_client_toolkit::reexports::protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_v1::WpAlphaModifierV1;
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::wlr_layer::LayerShell;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::subcompositor::SubcompositorState;
use tracing::{info, warn};

pub use error::WaylandError;
pub use probe::{Probe, ProbedOutput, probe};

use crate::activity::Presence;
use crate::config::{Config, Theme};
use crate::render::Frame;
use crate::render::image::Overlays;
use handlers::NoEvents;
use scale::Scale;
use surface::{ImageGlobals, OutputSurface};

pub use surface::ImageReady;

/// the compositor connection and every overlay surface
pub struct Wayland {
    connection: Connection,
    registry: RegistryState,
    outputs: OutputState,
    compositor: CompositorState,
    shm: Shm,
    layer_shell: LayerShell,
    viewporter: Option<WpViewporter>,
    fractional: Option<WpFractionalScaleManagerV1>,
    subcompositor: Option<SubcompositorState>,
    alpha: Option<WpAlphaModifierV1>,
    images: Overlays,
    image_jobs: Option<Sender<ImageReady>>,
    config: Config,
    theme: Option<Theme>,
    surfaces: Vec<OutputSurface>,
    closed: Vec<WlOutput>,
    warned: Vec<String>,
    ready: bool,
    queue: QueueHandle<Self>,
    format: wl_shm::Format,
}

impl Wayland {
    pub fn connect(
        config: Config,
        theme: Option<Theme>,
    ) -> Result<(Self, EventQueue<Self>), WaylandError> {
        let connection = Connection::connect_to_env()?;
        let (globals, mut queue) = registry_queue_init::<Self>(&connection)?;
        let qh = queue.handle();
        let missing = |name| move |source| WaylandError::Missing { name, source };
        let compositor = CompositorState::bind(&globals, &qh).map_err(missing("wl_compositor"))?;
        let shm = Shm::bind(&globals, &qh).map_err(missing("wl_shm"))?;
        let layer_shell =
            LayerShell::bind(&globals, &qh).map_err(|_| WaylandError::NoLayerShell)?;
        let viewporter = globals.bind(&qh, 1..=1, NoEvents).ok();
        let fractional = globals.bind(&qh, 1..=1, NoEvents).ok();
        let subcompositor =
            SubcompositorState::bind(compositor.wl_compositor().clone(), &globals, &qh).ok();
        let alpha = globals.bind(&qh, 1..=1, NoEvents).ok();
        info!(
            "wayland: fractional scale {}",
            match (&viewporter, &fractional) {
                (Some(_), Some(_)) => "supported",
                _ => "not supported, using integer scale",
            }
        );

        let mut wayland = Self {
            registry: RegistryState::new(&globals),
            outputs: OutputState::new(&globals, &qh),
            connection,
            compositor,
            shm,
            layer_shell,
            viewporter,
            fractional,
            subcompositor,
            alpha,
            images: Overlays::default(),
            image_jobs: None,
            config,
            theme,
            surfaces: Vec::new(),
            closed: Vec::new(),
            warned: Vec::new(),
            ready: false,
            queue: qh.clone(),
            format: wl_shm::Format::Argb8888,
        };
        // outputs send their names and sizes, and shm its formats, in answer
        // to being bound
        queue.roundtrip(&mut wayland)?;
        if wayland.shm.formats().contains(&wl_shm::Format::Abgr8888) {
            wayland.format = wl_shm::Format::Abgr8888;
        }
        info!("wayland: drawing into {:?} buffers", wayland.format);
        wayland.ready = true;
        wayland.reconcile(&qh, None);
        Ok((wayland, queue))
    }

    pub fn insert_source<D: AsMut<Self> + 'static>(
        &self,
        queue: EventQueue<Self>,
        handle: &LoopHandle<'static, D>,
        mut after: impl FnMut(&mut D) + 'static,
    ) -> Result<RegistrationToken, calloop::Error> {
        let connection = self.connection.clone();
        let source = WaylandSource::new(self.connection.clone(), queue);
        handle
            .insert_source(source, move |(), queue, data| {
                let dispatched = queue.dispatch_pending(data.as_mut());
                if let Some(err) = connection.protocol_error() {
                    return Err(DispatchError::Backend(backend::WaylandError::Protocol(err)));
                }
                after(data);
                dispatched
            })
            .map_err(|err| err.error)
    }

    /// fades every surface towards active at now, mapping and unmapping
    /// them as needed
    pub fn set_active(&mut self, active: bool, now: Instant) -> Presence {
        let mut presence = Presence::default();
        for surface in &mut self.surfaces {
            surface.set_active(active, now);
            presence.shown |= surface.is_shown();
            presence.fading |= surface.is_fading(now);
            presence.animated |= surface.is_animated();
        }
        presence
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.surfaces
            .iter()
            .filter_map(OutputSurface::deadline)
            .min()
    }

    pub fn render(&mut self, frame: &Frame<'_>) -> usize {
        let mut drawn = 0;
        for surface in &mut self.surfaces {
            let overlay = self.images.of(surface.entry());
            surface.update_image(overlay, self.image_jobs.as_ref());
            drawn += usize::from(surface.render(frame, &self.shm, &self.queue));
        }
        drawn
    }

    pub fn set_image_jobs(&mut self, jobs: Sender<ImageReady>) {
        self.image_jobs = Some(jobs);
    }

    /// the image overlays to show from now on
    pub fn set_images(&mut self, images: Overlays) {
        self.images = images;
    }

    /// a worker's scaled image, for the surface that asked for it
    pub fn image_ready(&mut self, ready: ImageReady) {
        let globals = ImageGlobals {
            compositor: &self.compositor,
            subcompositor: self.subcompositor.as_ref(),
            viewporter: self.viewporter.as_ref(),
            alpha: self.alpha.as_ref(),
            shm: &self.shm,
            format: self.format,
        };
        let surface = self
            .surfaces
            .iter_mut()
            .find(|surface| surface.surface_id() == *ready.surface());
        if let Some(surface) = surface {
            surface.show_image(&globals, &self.queue, ready);
        }
    }

    pub fn check_connection(&self) -> Result<(), WaylandError> {
        if let Some(err) = self.connection.protocol_error() {
            return Err(WaylandError::Lost(err.to_string()));
        }
        self.connection
            .flush()
            .map_err(|err| WaylandError::Lost(err.to_string()))
    }

    /// destroys every surface and sends the requests before disconnecting
    pub fn shutdown(&mut self) {
        self.surfaces.clear();
        if let Err(err) = self.connection.flush() {
            warn!("could not flush the Wayland connection: {err}");
        }
    }

    fn surface_mut(&mut self, surface: &ObjectId) -> Option<&mut OutputSurface> {
        self.surfaces
            .iter_mut()
            .find(|candidate| candidate.surface_id() == *surface)
    }

    fn scale_changed(&mut self, surface: &ObjectId, scale: Scale) {
        if let Some(surface) = self.surface_mut(surface) {
            surface.set_scale(scale);
        }
    }
}
