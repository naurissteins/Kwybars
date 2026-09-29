//! keeping one surface on each selected output as outputs come and go

use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::reexports::client::{Proxy, QueueHandle};
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use tracing::{info, warn};

use super::Wayland;
use super::selection;
use super::surface::{Globals, OutputSurface};

impl Wayland {
    /// creates and removes surfaces so the selected outputs have one each;
    /// `gone` is an output that is being removed
    pub(super) fn reconcile(&mut self, qh: &QueueHandle<Self>, gone: Option<&WlOutput>) {
        if !self.ready {
            return;
        }
        // an output is listed as soon as it is bound, but its name and size
        // only count once the compositor finished describing it
        let (outputs, infos): (Vec<WlOutput>, Vec<_>) = self
            .outputs
            .outputs()
            .filter(|output| Some(output) != gone && !self.closed.contains(output))
            .filter_map(|output| {
                let info = self.outputs.info(&output)?;
                Some((output, info))
            })
            .unzip();
        let names: Vec<Option<&str>> = infos.iter().map(|info| info.name.as_deref()).collect();
        let selection = selection::select(&self.config.overlay, &names);
        for warning in &selection.warnings {
            warn!("{warning}");
        }
        let wanted: Vec<_> = selection
            .targets
            .iter()
            .filter_map(|target| Some((outputs.get(target.output)?, target.entry)))
            .collect();

        self.surfaces.retain(|surface| {
            let keep = wanted
                .iter()
                .any(|(output, entry)| surface.is_for(output, *entry));
            if !keep {
                info!("removing the overlay from {}", surface.label());
            }
            keep
        });
        for (output, entry) in wanted {
            if !self
                .surfaces
                .iter()
                .any(|surface| surface.is_for(output, entry))
            {
                let surface = self.create_surface(qh, output, entry);
                self.surfaces.push(surface);
            }
        }
        for surface in &mut self.surfaces {
            let size = self.outputs.info(surface.output()).and_then(|info| {
                let (width, height) = info.logical_size?;
                Some((u32::try_from(width).ok()?, u32::try_from(height).ok()?))
            });
            surface.set_output_size(size);
        }
    }

    fn create_surface(
        &self,
        qh: &QueueHandle<Self>,
        output: &WlOutput,
        entry: Option<usize>,
    ) -> OutputSurface {
        let output_entry = entry.and_then(|index| self.config.overlay.outputs.get(index));
        let config = self.config.surface(output_entry, self.theme.as_ref());
        let globals = Globals {
            compositor: &self.compositor,
            layer_shell: &self.layer_shell,
            viewporter: self.viewporter.as_ref(),
            fractional: self.fractional.as_ref(),
            format: self.format,
        };
        OutputSurface::new(
            &globals,
            qh,
            output,
            entry,
            self.output_label(output),
            config,
        )
    }

    /// the connector name, else make and model, for logs
    pub(super) fn output_label(&self, output: &WlOutput) -> String {
        let Some(info) = self.outputs.info(output) else {
            return format!("output {}", output.id().protocol_id());
        };
        match info.name {
            Some(name) if !name.is_empty() => name,
            _ => format!("{} {}", info.make, info.model).trim().to_owned(),
        }
    }

    /// the compositor closed layer, for example because its output is going away
    pub(super) fn surface_closed(&mut self, layer: &LayerSurface) {
        let Some(index) = self
            .surfaces
            .iter()
            .position(|surface| surface.is_layer(layer))
        else {
            return;
        };
        let surface = self.surfaces.swap_remove(index);
        info!("the compositor closed the overlay on {}", surface.label());
        self.closed.push(surface.output().clone());
    }

    /// forgets output once the compositor removed it
    pub(super) fn output_removed(&mut self, qh: &QueueHandle<Self>, output: &WlOutput) {
        self.closed.retain(|closed| closed != output);
        // sctk still lists the output during this call
        self.reconcile(qh, Some(output));
    }
}
