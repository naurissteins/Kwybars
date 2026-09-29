//! one layer surface on one output, with its scale and shared-memory buffers

mod buffers;
mod draw;
mod layer;
mod settings;
mod visibility;

use std::time::Duration;

use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::reexports::client::protocol::{
    wl_output::WlOutput, wl_shm, wl_surface::WlSurface,
};
use smithay_client_toolkit::reexports::client::{Proxy, QueueHandle, backend::ObjectId};
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::WpFractionalScaleV1,
};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{LayerShell, LayerSurface};
use tracing::debug;

use super::Wayland;
use super::handlers::{NoEvents, ScaleData};
use super::placement::Placement;
use super::scale::Scale;
use crate::activity::Fade;
use crate::config::SurfaceConfig;
use crate::render::{ByteOrder, Painter};
use buffers::BufferRing;
use layer::create_layer;

/// layer-shell namespace compositors can match rules on
const NAMESPACE: &str = "kwybars";

/// protocol objects a surface is created from
pub struct Globals<'a> {
    pub compositor: &'a CompositorState,
    pub layer_shell: &'a LayerShell,
    pub viewporter: Option<&'a WpViewporter>,
    pub fractional: Option<&'a WpFractionalScaleManagerV1>,
    /// the shm format buffers use
    pub format: wl_shm::Format,
}

/// the overlay on one output
pub struct OutputSurface {
    output: WlOutput,
    entry: Option<usize>,
    label: String,
    config: SurfaceConfig,
    placement: Placement,
    layer: LayerSurface,
    viewport: Option<WpViewport>,
    fractional_scale: Option<WpFractionalScaleV1>,
    scale: Scale,
    configured: Option<(u32, u32)>,
    output_size: Option<(u32, u32)>,

    applied: Option<(Scale, (u32, u32))>,
    order: ByteOrder,
    ring: BufferRing,
    painter: Option<Painter>,
    frame_pending: bool,
    drawn: Option<u64>,
    failed: bool,

    fade: Fade,
    /// committed since creation or the last unmap, so mapped or about to be
    shown: bool,
}

impl OutputSurface {
    pub fn new(
        globals: &Globals<'_>,
        qh: &QueueHandle<Wayland>,
        output: &WlOutput,
        entry: Option<usize>,
        label: String,
        config: SurfaceConfig,
    ) -> Self {
        let placement = Placement::new(&config.overlay, config.visualizer.layout);
        let wl_surface = globals.compositor.create_surface(qh);
        let (viewport, fractional_scale) = scale_objects(globals, qh, &wl_surface);
        let layer = create_layer(globals, qh, wl_surface, output, &placement, &label);

        Self {
            output: output.clone(),
            entry,
            label,
            placement,
            layer,
            scale: if viewport.is_some() {
                Scale::Fractional(120)
            } else {
                Scale::Integer(1)
            },
            viewport,
            fractional_scale,
            configured: None,
            output_size: None,
            applied: None,
            order: if globals.format == wl_shm::Format::Abgr8888 {
                ByteOrder::Rgba
            } else {
                ByteOrder::Bgra
            },
            ring: BufferRing::new(globals.format),
            painter: None,
            frame_pending: false,
            drawn: None,
            failed: false,
            fade: Fade::new(
                Duration::from_millis(config.overlay.fade_in_ms),
                Duration::from_millis(config.overlay.fade_out_ms),
            ),
            shown: false,
            config,
        }
    }

    /// whether this surface is the one `output` and `entry` ask for
    pub fn is_for(&self, output: &WlOutput, entry: Option<usize>) -> bool {
        self.output == *output && self.entry == entry
    }

    /// the `[[overlay.outputs]]` entry this surface was selected by
    pub fn entry(&self) -> Option<usize> {
        self.entry
    }

    pub fn is_layer(&self, layer: &LayerSurface) -> bool {
        self.layer == *layer
    }

    pub fn surface_id(&self) -> ObjectId {
        self.layer.wl_surface().id()
    }

    pub fn output(&self) -> &WlOutput {
        &self.output
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    /// the compositor sent a size; the next render redraws when it changed
    pub fn configure(&mut self, size: (u32, u32)) {
        debug!("{}: configured {}x{}", self.label, size.0, size.1);
        if self.configured != Some(size) {
            self.configured = Some(size);
            self.drawn = None;
        }
    }

    pub fn set_output_size(&mut self, size: Option<(u32, u32)>) {
        if self.output_size != size {
            self.output_size = size;
            self.drawn = None;
        }
    }

    /// a preferred scale from the compositor; integer scales are ignored
    /// while fractional scaling is in use
    pub fn set_scale(&mut self, scale: Scale) {
        let fractional = matches!(self.scale, Scale::Fractional(_));
        if matches!(scale, Scale::Fractional(_)) != fractional || self.scale == scale {
            return;
        }
        self.scale = scale;
        self.drawn = None;
    }

    /// the frame callback requested with the last commit arrived
    pub fn frame_done(&mut self) {
        self.frame_pending = false;
    }

    /// logical size and buffer size, once configured
    fn sizes(&self) -> Option<((u32, u32), (u32, u32))> {
        let logical = self
            .placement
            .surface_size(self.configured?, self.output_size);
        Some((logical, self.scale.buffer_size(logical)))
    }
}

/// the viewport and fractional scale objects, when the compositor has both
fn scale_objects(
    globals: &Globals<'_>,
    qh: &QueueHandle<Wayland>,
    wl_surface: &WlSurface,
) -> (Option<WpViewport>, Option<WpFractionalScaleV1>) {
    let (Some(viewporter), Some(fractional)) = (globals.viewporter, globals.fractional) else {
        return (None, None);
    };
    let data = ScaleData {
        surface: wl_surface.id(),
    };
    (
        Some(viewporter.get_viewport(wl_surface, qh, NoEvents)),
        Some(fractional.get_fractional_scale(wl_surface, qh, data)),
    )
}

impl Drop for OutputSurface {
    fn drop(&mut self) {
        // these have destructor requests; the layer surface destroys itself
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
        if let Some(fractional_scale) = self.fractional_scale.take() {
            fractional_scale.destroy();
        }
    }
}
