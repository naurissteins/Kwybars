//! wayland connection, outputs, and one layer surface per selected output

mod error;
mod handlers;
mod outputs;
mod placement;
mod scale;
mod selection;
mod surface;

use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::{LoopHandle, RegistrationToken};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::backend::ObjectId;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::{wl_output::WlOutput, wl_shm};
use smithay_client_toolkit::reexports::client::{Connection, EventQueue, QueueHandle};
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::wlr_layer::LayerShell;
use smithay_client_toolkit::shm::Shm;
use tracing::{info, warn};

pub use error::WaylandError;

use crate::config::{Config, Theme};
use crate::render::Frame;
use handlers::NoEvents;
use scale::Scale;
use surface::OutputSurface;

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
    config: Config,
    theme: Option<Theme>,
    surfaces: Vec<OutputSurface>,
    closed: Vec<WlOutput>,
    ready: bool,
    queue: QueueHandle<Self>,
    format: wl_shm::Format,
}

impl Wayland {
    /// connects, binds the globals, and creates the surfaces for the outputs
    /// present now; nothing is shown until the event queue is dispatched
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
            config,
            theme,
            surfaces: Vec::new(),
            closed: Vec::new(),
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

    /// dispatches wayland events on `handle`'s loop, then calls `after` so
    /// surfaces whose frame callback arrived can draw; `D` gives access to self
    pub fn insert_source<D: AsMut<Self> + 'static>(
        &self,
        queue: EventQueue<Self>,
        handle: &LoopHandle<'static, D>,
        mut after: impl FnMut(&mut D) + 'static,
    ) -> Result<RegistrationToken, calloop::Error> {
        let source = WaylandSource::new(self.connection.clone(), queue);
        handle
            .insert_source(source, move |(), queue, data| {
                let dispatched = queue.dispatch_pending(data.as_mut());
                after(data);
                dispatched
            })
            .map_err(|err| err.error)
    }

    /// shows `frame` on every surface that is ready for it; returns how many
    /// committed a new buffer
    pub fn render(&mut self, frame: &Frame<'_>) -> usize {
        let mut drawn = 0;
        for surface in &mut self.surfaces {
            drawn += usize::from(surface.render(frame, &self.shm, &self.queue));
        }
        drawn
    }

    /// explains an event loop failure when the compositor connection is what
    /// failed: a protocol error, or a socket that no longer accepts requests
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
