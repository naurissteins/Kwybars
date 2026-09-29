//! one layer surface on one output, with its scale and shared-memory buffer

mod fill;

use smithay_client_toolkit::compositor::{CompositorState, Region};
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::reexports::client::{Proxy, QueueHandle, backend::ObjectId};
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::WpFractionalScaleV1,
};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer as ShellLayer, LayerShell, LayerSurface,
};
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};

use fill::DrawError;
use tracing::{debug, info, warn};

use super::Wayland;
use super::handlers::{NoEvents, ScaleData};
use super::placement::{Anchors, Placement};
use super::scale::Scale;
use crate::config::{Layer, SurfaceConfig};

/// layer-shell namespace compositors can match rules on
const NAMESPACE: &str = "kwybars";

/// protocol objects a surface is created from
pub struct Globals<'a> {
    pub compositor: &'a CompositorState,
    pub layer_shell: &'a LayerShell,
    pub viewporter: Option<&'a WpViewporter>,
    pub fractional: Option<&'a WpFractionalScaleManagerV1>,
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
    pool: Option<SlotPool>,
    buffer: Option<Buffer>,
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
        let (viewport, fractional_scale) = match (globals.viewporter, globals.fractional) {
            (Some(viewporter), Some(fractional)) => (
                Some(viewporter.get_viewport(&wl_surface, qh, NoEvents)),
                Some(fractional.get_fractional_scale(
                    &wl_surface,
                    qh,
                    ScaleData {
                        surface: wl_surface.id(),
                    },
                )),
            ),
            _ => (None, None),
        };
        let layer = globals.layer_shell.create_layer_surface(
            qh,
            wl_surface,
            shell_layer(placement.layer),
            Some(NAMESPACE),
            Some(output),
        );
        layer.set_anchor(anchor(placement.anchors));
        let margins = placement.margins;
        layer.set_margin(margins.top, margins.right, margins.bottom, margins.left);
        layer.set_size(placement.width, placement.height);
        layer.set_exclusive_zone(0);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        // an empty input region lets every click through to what is below
        match Region::new(globals.compositor) {
            Ok(region) => layer
                .wl_surface()
                .set_input_region(Some(region.wl_region())),
            Err(err) => warn!("could not make {label} click-through: {err}"),
        }
        // the first commit has no buffer and asks for a configure
        layer.commit();
        info!(
            "overlay on {label}: {:?} layer, anchors {:?}, margins {:?}, size {}x{} (0 stretches)",
            placement.layer,
            placement.anchors,
            placement.margins,
            placement.width,
            placement.height
        );

        Self {
            output: output.clone(),
            entry,
            label,
            config,
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
            pool: None,
            buffer: None,
        }
    }

    /// whether this surface is the one `output` and `entry` ask for
    pub fn is_for(&self, output: &WlOutput, entry: Option<usize>) -> bool {
        self.output == *output && self.entry == entry
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

    /// the compositor sent a size; draws when anything visible changed
    pub fn configure(&mut self, size: (u32, u32), shm: &Shm) {
        debug!("{}: configured {}x{}", self.label, size.0, size.1);
        if self.configured == Some(size) {
            return;
        }
        self.configured = Some(size);
        self.draw(shm);
    }

    pub fn set_output_size(&mut self, size: Option<(u32, u32)>) {
        self.output_size = size;
    }

    /// a preferred scale from the compositor; integer scales are ignored
    /// while fractional scaling is in use
    pub fn set_scale(&mut self, scale: Scale, shm: &Shm) {
        let fractional = matches!(self.scale, Scale::Fractional(_));
        if matches!(scale, Scale::Fractional(_)) != fractional || self.scale == scale {
            return;
        }
        self.scale = scale;
        self.draw(shm);
    }

    /// logical size and buffer size, once configured
    fn sizes(&self) -> Option<((u32, u32), (u32, u32))> {
        let logical = self
            .placement
            .surface_size(self.configured?, self.output_size);
        Some((logical, self.scale.buffer_size(logical)))
    }

    /// fills the surface with the translucent placeholder
    fn draw(&mut self, shm: &Shm) {
        // attaching a buffer before the first configure is a protocol error
        let Some((logical, size)) = self.sizes() else {
            return;
        };
        if let Err(err) = self.present(shm, logical, size) {
            warn!("could not draw the overlay on {}: {err}", self.label);
            return;
        }
        debug!(
            "{}: {}x{} logical, {}x{} buffer, scale {}",
            self.label, logical.0, logical.1, size.0, size.1, self.scale
        );
    }

    fn present(
        &mut self,
        shm: &Shm,
        logical: (u32, u32),
        size: (u32, u32),
    ) -> Result<(), DrawError> {
        let buffer = fill::buffer(&mut self.pool, shm, size, fill::color(&self.config))?;
        let surface = self.layer.wl_surface();
        match (&self.viewport, self.scale) {
            (Some(viewport), Scale::Fractional(_)) => {
                let (Ok(width), Ok(height)) = (i32::try_from(logical.0), i32::try_from(logical.1))
                else {
                    return Err(DrawError::TooLarge);
                };
                surface.set_buffer_scale(1);
                viewport.set_destination(width, height);
            }
            (_, Scale::Integer(scale)) => {
                surface.set_buffer_scale(i32::try_from(scale).map_err(|_| DrawError::TooLarge)?);
            }
            (None, Scale::Fractional(_)) => surface.set_buffer_scale(1),
        }
        buffer.attach_to(surface)?;
        surface.damage_buffer(0, 0, buffer.stride() / 4, buffer.height());
        self.layer.commit();
        // the previous buffer goes back to the pool once the compositor releases it
        self.buffer = Some(buffer);
        Ok(())
    }
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

fn shell_layer(layer: Layer) -> ShellLayer {
    match layer {
        Layer::Background => ShellLayer::Background,
        Layer::Bottom => ShellLayer::Bottom,
        Layer::Top => ShellLayer::Top,
    }
}

fn anchor(anchors: Anchors) -> Anchor {
    let mut anchor = Anchor::empty();
    anchor.set(Anchor::TOP, anchors.top);
    anchor.set(Anchor::BOTTOM, anchors.bottom);
    anchor.set(Anchor::LEFT, anchors.left);
    anchor.set(Anchor::RIGHT, anchors.right);
    anchor
}
