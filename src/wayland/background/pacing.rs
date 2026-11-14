use std::time::{Duration, Instant};

use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::reexports::client::QueueHandle;
use smithay_client_toolkit::shell::WaylandSurface;

use super::{Surface, Wayland};
use crate::render::background::{Crop, crop};
use crate::wayland::surface::buffers::DrawError;

impl Surface {
    pub(super) fn render(
        &mut self,
        now: Instant,
        bass: f32,
        interval: Duration,
        queue: &QueueHandle<Wayland>,
    ) -> Result<(), DrawError> {
        let (Some(screen), Some(upload)) = (self.screen, self.upload.as_ref()) else {
            return Ok(());
        };
        if self.pending {
            return Ok(());
        }
        if let Some(due) = self
            .last_frame
            .map(|last| last + interval)
            .filter(|due| *due > now)
        {
            self.due = Some(due);
            return Ok(());
        }
        self.due = None;
        let pose = self.motion.pose(&self.config, bass, now);
        self.animated = pose.animated;
        let next = crop(upload.source.size(), screen, pose.zoom, pose.offset)
            .ok_or(DrawError::TooLarge)?;
        if self.previous != Some(next) {
            let Crop(x, y, width, height) = next;
            self.viewport.set_source(x, y, width, height);
            // configure accepted only positive sizes within i32
            self.viewport
                .set_destination(screen.0 as i32, screen.1 as i32);
            if !self.attached {
                self.layer.wl_surface().attach(Some(&upload.buffer), 0, 0);
                self.layer.wl_surface().damage_buffer(
                    0,
                    0,
                    upload.source.size().0 as i32,
                    upload.source.size().1 as i32,
                );
                self.attached = true;
            }
            self.previous = Some(next);
        } else if !self.animated {
            return Ok(());
        }
        if self.animated {
            self.layer
                .wl_surface()
                .frame(queue, FrameCallbackData(self.layer.wl_surface().clone()));
            self.pending = true;
        }
        self.layer.commit();
        self.last_frame = Some(now);
        Ok(())
    }
}
