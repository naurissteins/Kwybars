use std::sync::Arc;

use smithay_client_toolkit::reexports::client::backend::ObjectId;
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::reexports::client::{Proxy, QueueHandle};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;

use super::{Surface, Wayland};
use crate::config::BackgroundSource;

impl Wayland {
    pub(in crate::wayland) fn reconcile_backgrounds(
        &mut self,
        wanted: &[(&WlOutput, Option<usize>)],
        queue: &QueueHandle<Self>,
    ) {
        self.background
            .closed
            .retain(|closed| self.outputs.info(closed).is_some());
        let following: Vec<_> = wanted
            .iter()
            .filter_map(|(output, entry)| {
                let section = entry.and_then(|entry| self.config.overlay.outputs.get(entry));
                let settings = self.config.background(section);
                (settings.enabled && settings.source == BackgroundSource::Sweetbg)
                    .then(|| self.output_label(output))
            })
            .collect();
        if self.background.following != following {
            self.background.following = following;
            self.background.selection = self.background.selection.wrapping_add(1);
        }
        let mut wanted: Vec<_> = wanted
            .iter()
            .filter_map(|(output, entry)| {
                let section = entry.and_then(|entry| self.config.overlay.outputs.get(entry));
                let config = self.config.background(section);
                let source = match config.source {
                    BackgroundSource::File => {
                        Arc::clone(&self.background.images.of(*entry)?.source)
                    }
                    BackgroundSource::Sweetbg => {
                        let label = self.output_label(output);
                        Arc::clone(
                            &self
                                .background
                                .external
                                .iter()
                                .find(|(name, _)| *name == label)?
                                .1,
                        )
                    }
                };
                (config.enabled && !self.background.closed.contains(output))
                    .then_some((*output, *entry, config, source))
            })
            .collect();
        if self.viewporter.is_none() {
            if !wanted.is_empty() && !self.background.warned {
                tracing::warn!(
                    "reactive backgrounds require wp_viewporter; bars continue without them"
                );
                self.background.warned = true;
            }
            wanted.clear();
        }
        self.background.surfaces.retain(|surface| {
            wanted
                .iter()
                .any(|(output, entry, _, _)| surface.output == **output && surface.entry == *entry)
        });
        for (output, entry, config, source) in wanted {
            if let Some(surface) = self
                .background
                .surfaces
                .iter_mut()
                .find(|surface| surface.output == *output && surface.entry == entry)
            {
                surface.granted = self.background.grants.contains(&surface.label);
                surface.config = config;
                surface.source = Arc::clone(&source);
                continue;
            }
            let Some(viewporter) = self.viewporter.as_ref() else {
                continue;
            };
            match Surface::new(
                &self.compositor,
                &self.layer_shell,
                viewporter,
                self.fractional.as_ref(),
                queue,
                output,
                entry,
                self.output_label(output),
                config,
                Arc::clone(&source),
            ) {
                Ok(mut surface) => {
                    surface.granted = self.background.grants.contains(&surface.label);
                    self.background.surfaces.push(surface);
                }
                Err(error) => tracing::warn!(
                    "could not create background on {}: {error}",
                    self.output_label(output)
                ),
            }
        }
    }

    pub(in crate::wayland) fn background_scale_changed(
        &mut self,
        id: &ObjectId,
        scale: crate::wayland::scale::Scale,
    ) {
        if let Some(surface) = self
            .background
            .surfaces
            .iter_mut()
            .find(|surface| surface.layer.wl_surface().id() == *id)
        {
            let fractional = surface.fractional.is_some();
            if matches!(scale, crate::wayland::scale::Scale::Fractional(_)) == fractional {
                surface.scale = scale;
            }
        }
    }

    pub(in crate::wayland) fn background_frame(&mut self, surface: &ObjectId) {
        if let Some(surface) = self
            .background
            .surfaces
            .iter_mut()
            .find(|item| item.layer.wl_surface().id() == *surface)
        {
            surface.pending = false;
        }
    }

    pub(in crate::wayland) fn background_configure(
        &mut self,
        layer: &LayerSurface,
        size: (u32, u32),
    ) {
        if let Some(surface) = self
            .background
            .surfaces
            .iter_mut()
            .find(|item| item.layer == *layer)
            && size.0 > 0
            && size.1 > 0
            && i32::try_from(size.0).is_ok()
            && i32::try_from(size.1).is_ok()
        {
            surface.screen = Some(size);
            surface.previous = None;
        }
    }

    pub(in crate::wayland) fn background_closed(&mut self, layer: &LayerSurface) {
        if let Some(index) = self
            .background
            .surfaces
            .iter()
            .position(|item| item.layer == *layer)
        {
            let surface = self.background.surfaces.swap_remove(index);
            self.background.closed.push(surface.output.clone());
        }
    }
}
