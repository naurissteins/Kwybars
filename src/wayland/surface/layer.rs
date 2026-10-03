//! layer-shell state from the surface placement

use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer as ShellLayer, LayerSurface,
};

use smithay_client_toolkit::compositor::Region;
use smithay_client_toolkit::reexports::client::protocol::{
    wl_output::WlOutput, wl_surface::WlSurface,
};
use smithay_client_toolkit::reexports::client::{Proxy, QueueHandle};
use smithay_client_toolkit::shell::WaylandSurface;
use tracing::{info, warn};

use super::{Globals, NAMESPACE, OutputSurface};
use crate::config::Layer;
use crate::wayland::Wayland;
use crate::wayland::handlers::SyncData;
use crate::wayland::placement::{Anchors, Placement};

/// makes wl_surface a click-through layer surface on output, nothing
/// is committed yet
pub(super) fn create_layer(
    globals: &Globals<'_>,
    qh: &QueueHandle<Wayland>,
    wl_surface: WlSurface,
    output: &WlOutput,
    placement: &Placement,
    label: &str,
) -> LayerSurface {
    let layer = globals.layer_shell.create_layer_surface(
        qh,
        wl_surface,
        shell_layer(placement.layer),
        Some(NAMESPACE),
        Some(output),
    );
    place(&layer, placement);
    // an empty input region lets every click through to what is below
    match Region::new(globals.compositor) {
        Ok(region) => layer
            .wl_surface()
            .set_input_region(Some(region.wl_region())),
        Err(err) => warn!("could not make {label} click-through: {err}"),
    }
    info!(
        "overlay on {label}: {:?} layer, anchors {:?}, margins {:?}, size {}x{} (0 stretches)",
        placement.layer, placement.anchors, placement.margins, placement.width, placement.height
    );
    layer
}

impl OutputSurface {
    pub(super) fn commit_placement(&mut self) {
        place(&self.layer, &self.placement);
        self.layer.commit();
        let data = SyncData {
            surface: self.layer.wl_surface().id(),
        };
        self.display.sync(&self.queue, data);
        self.syncs += 1;
    }
}

/// sends the layer state, which an unmap resets
pub(super) fn place(layer: &LayerSurface, placement: &Placement) {
    layer.set_anchor(anchor(placement.anchors));
    let margins = placement.margins;
    layer.set_margin(margins.top, margins.right, margins.bottom, margins.left);
    layer.set_size(placement.width, placement.height);
    layer.set_exclusive_zone(0);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
}

pub(super) fn shell_layer(layer: Layer) -> ShellLayer {
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
