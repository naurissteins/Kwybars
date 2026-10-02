//! sctk handler traits and the protocol objects sctk does not cover

use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::dispatch2::Dispatch2;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::backend::ObjectId;
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_surface};
use smithay_client_toolkit::reexports::client::{Connection, Proxy, QueueHandle};
use smithay_client_toolkit::reexports::protocols::wp::alpha_modifier::v1::client::{
    wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1, wp_alpha_modifier_v1::WpAlphaModifierV1,
};
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::{self, WpFractionalScaleV1},
};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::wlr_layer::{
    LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_dispatch2, delegate_registry, registry_handlers};
use tracing::debug;

use super::Wayland;
use super::scale::Scale;

/// user data for objects that send no events
pub struct NoEvents;

/// user data of a surface's wp_fractional_scale_v1
pub struct ScaleData {
    pub surface: ObjectId,
}

impl Dispatch2<WpViewporter, Wayland> for NoEvents {
    fn event(
        &self,
        _: &mut Wayland,
        _: &WpViewporter,
        _: <WpViewporter as Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<Wayland>,
    ) {
    }
}

impl Dispatch2<WpViewport, Wayland> for NoEvents {
    fn event(
        &self,
        _: &mut Wayland,
        _: &WpViewport,
        _: <WpViewport as Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<Wayland>,
    ) {
    }
}

impl Dispatch2<WpAlphaModifierV1, Wayland> for NoEvents {
    fn event(
        &self,
        _: &mut Wayland,
        _: &WpAlphaModifierV1,
        _: <WpAlphaModifierV1 as Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<Wayland>,
    ) {
    }
}

impl Dispatch2<WpAlphaModifierSurfaceV1, Wayland> for NoEvents {
    fn event(
        &self,
        _: &mut Wayland,
        _: &WpAlphaModifierSurfaceV1,
        _: <WpAlphaModifierSurfaceV1 as Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<Wayland>,
    ) {
    }
}

impl Dispatch2<WpFractionalScaleManagerV1, Wayland> for NoEvents {
    fn event(
        &self,
        _: &mut Wayland,
        _: &WpFractionalScaleManagerV1,
        _: <WpFractionalScaleManagerV1 as Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<Wayland>,
    ) {
    }
}

impl Dispatch2<WpFractionalScaleV1, Wayland> for ScaleData {
    fn event(
        &self,
        state: &mut Wayland,
        _: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _: &Connection,
        _: &QueueHandle<Wayland>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            state.scale_changed(&self.surface, Scale::Fractional(scale));
        }
    }
}

impl CompositorHandler for Wayland {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        let factor = u32::try_from(new_factor).unwrap_or(1).max(1);
        self.scale_changed(&surface.id(), Scale::Integer(factor));
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        if let Some(surface) = self.surface_mut(&surface.id()) {
            surface.frame_done();
        }
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for Wayland {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        debug!("output added: {}", self.output_label(&output));
        self.reconcile(qh, None);
    }

    fn update_output(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        debug!("output changed: {}", self.output_label(&output));
        self.reconcile(qh, None);
    }

    fn output_destroyed(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        debug!("output removed: {}", self.output_label(&output));
        self.output_removed(qh, &output);
    }
}

impl LayerShellHandler for Wayland {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        self.surface_closed(layer);
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        if let Some(surface) = self
            .surfaces
            .iter_mut()
            .find(|surface| surface.is_layer(layer))
        {
            surface.configure(configure.new_size);
        }
    }
}

impl ShmHandler for Wayland {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for Wayland {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    registry_handlers![OutputState];
}

delegate_registry!(Wayland);
delegate_dispatch2!(Wayland);
