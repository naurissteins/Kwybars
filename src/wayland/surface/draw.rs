//! showing frames: pacing by frame callbacks, patching buffers, damage

use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::reexports::client::QueueHandle;
use smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::Shm;
use tracing::{debug, warn};

use super::OutputSurface;
use super::buffers::DrawError;
use crate::render::{Canvas, Frame, Painter};
use crate::wayland::Wayland;
use crate::wayland::scale::Scale;

impl OutputSurface {
    pub fn render(&mut self, frame: &Frame<'_>, shm: &Shm, qh: &QueueHandle<Wayland>) -> bool {
        // attaching a buffer before the first configure is a protocol error
        let Some((logical, size)) = self.sizes() else {
            return false;
        };
        if self.frame_pending {
            return false;
        }
        if self.drawn == Some(frame.generation) || !self.layout(frame, size) {
            self.drawn = Some(frame.generation);
            if frame.animating {
                self.request_frame(qh);
                self.layer.commit();
            }
            return false;
        }
        self.drawn = Some(frame.generation);
        match self.draw(shm, logical, size) {
            Ok(true) => {
                self.failed = false;
                if frame.animating {
                    self.request_frame(qh);
                }
                self.layer.commit();
                true
            }
            Ok(false) => {
                // every buffer is still with the compositor, retry on the next callback
                self.drawn = None;
                self.request_frame(qh);
                self.layer.commit();
                false
            }
            Err(err) => {
                if !self.failed {
                    warn!("could not draw the overlay on {}: {err}", self.label);
                    self.failed = true;
                }
                false
            }
        }
    }

    /// lays out the bars for `size`; false when nothing on screen changes
    fn layout(&mut self, frame: &Frame<'_>, size: (u32, u32)) -> bool {
        let scale = self.scale.factor();
        if !self
            .painter
            .as_ref()
            .is_some_and(|painter| painter.fits(size, scale))
        {
            self.painter = Some(Painter::new(
                &self.config,
                frame.heights.len(),
                size,
                scale,
                self.order,
            ));
        }
        self.painter
            .as_mut()
            .is_some_and(|painter| painter.layout(frame.heights))
    }

    fn draw(
        &mut self,
        shm: &Shm,
        logical: (u32, u32),
        size: (u32, u32),
    ) -> Result<bool, DrawError> {
        let Some(painter) = self.painter.as_mut() else {
            return Ok(false);
        };
        let Some((slot, bytes)) = self.ring.acquire(shm, size, || painter.new_contents())? else {
            return Ok(false);
        };
        let mut canvas = Canvas::new(bytes, size).ok_or(DrawError::TooLarge)?;
        painter.paint(&mut canvas, &mut slot.contents);

        let surface = self.layer.wl_surface();
        if self.applied != Some((self.scale, logical)) {
            apply_scale(surface, self.viewport.as_ref(), self.scale, logical)?;
            self.applied = Some((self.scale, logical));
            debug!(
                "{}: {}x{} logical, {}x{} buffer, scale {}",
                self.label, logical.0, logical.1, size.0, size.1, self.scale
            );
        }
        slot.buffer.attach_to(surface)?;
        painter.present(|area| {
            let int = |value: u32| i32::try_from(value).unwrap_or(i32::MAX);
            surface.damage_buffer(int(area.x), int(area.y), int(area.width), int(area.height));
        });
        Ok(true)
    }

    fn request_frame(&mut self, qh: &QueueHandle<Wayland>) {
        let surface = self.layer.wl_surface();
        surface.frame(qh, FrameCallbackData(surface.clone()));
        self.frame_pending = true;
    }
}

/// tells the compositor how buffer pixels map to the logical surface
fn apply_scale(
    surface: &WlSurface,
    viewport: Option<&WpViewport>,
    scale: Scale,
    (width, height): (u32, u32),
) -> Result<(), DrawError> {
    let int = |value: u32| i32::try_from(value).map_err(|_| DrawError::TooLarge);
    match (viewport, scale) {
        (Some(viewport), Scale::Fractional(_)) => {
            surface.set_buffer_scale(1);
            viewport.set_destination(int(width)?, int(height)?);
        }
        (_, Scale::Integer(scale)) => surface.set_buffer_scale(int(scale)?),
        (None, Scale::Fractional(_)) => surface.set_buffer_scale(1),
    }
    Ok(())
}
