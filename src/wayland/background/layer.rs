use std::sync::Arc;

use smithay_client_toolkit::compositor::{CompositorState, Region};
use smithay_client_toolkit::reexports::client::{QueueHandle, Proxy};
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell};

use super::{Surface, Wayland};
use crate::config::BackgroundConfig;
use crate::render::background::Motion;
use crate::render::image::Encoded;
use crate::wayland::handlers::NoEvents;
use crate::wayland::handlers::ScaleData;
use crate::wayland::scale::Scale;

impl Surface {
    #[expect(
        clippy::too_many_arguments,
        reason = "surface creation takes the protocol globals and resolved output settings"
    )]
    pub(super) fn new(
        compositor: &CompositorState,
        shell: &LayerShell,
        viewporter: &WpViewporter,
        fractional: Option<&WpFractionalScaleManagerV1>,
        queue: &QueueHandle<Wayland>,
        output: &WlOutput,
        entry: Option<usize>,
        label: String,
        config: BackgroundConfig,
        source: Arc<Encoded>,
    ) -> Result<Self, std::io::Error> {
        let surface = compositor.create_surface(queue);
        let region = Region::new(compositor).map_err(std::io::Error::other)?;
        surface.set_input_region(Some(region.wl_region()));
        let viewport = viewporter.get_viewport(&surface, queue, NoEvents);
        let fractional = fractional.map(|manager| {
            manager.get_fractional_scale(
                &surface,
                queue,
                ScaleData {
                    surface: surface.id(),
                },
            )
        });
        let scale = if fractional.is_some() {
            Scale::Fractional(120)
        } else {
            Scale::Integer(1)
        };
        let layer = shell.create_layer_surface(
            queue,
            surface,
            Layer::Background,
            Some("kwybars-background"),
            Some(output),
        );
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_size(0, 0);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.commit();
        tracing::info!("background on {label}");
        Ok(Self {
            layer,
            viewport,
            output: output.clone(),
            entry,
            label,
            config,
            source,
            fractional,
            scale,
            upload: None,
            screen: None,
            pending: false,
            attached: false,
            granted: false,
            previous: None,
            motion: Motion::default(),
            last_frame: None,
            due: None,
            retry_at: None,
            animated: false,
            failed: false,
        })
    }
}
