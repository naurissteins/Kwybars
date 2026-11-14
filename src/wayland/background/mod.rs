//! persistent wallpaper surfaces, independent of the bars' activity/fade

mod image;
mod layer;
mod pacing;
mod selection;

use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use calloop::channel::Sender;
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;

use super::Wayland;
use crate::config::{BackgroundConfig, BackgroundSource};
use crate::render::background::{Crop, Motion};
use crate::render::image::{Encoded, Overlays};
use crate::wayland::scale::Scale;

pub use image::Ready;
use image::{Upload, Uploads};

#[derive(Default)]
pub(super) struct Backgrounds {
    pub surfaces: Vec<Surface>,
    pub images: Overlays<Encoded>,
    uploads: Uploads,
    external: Vec<(String, Arc<Encoded>)>,
    grants: Vec<String>,
    following: Vec<String>,
    selection: u64,
    pub jobs: Option<Sender<Ready>>,
    pub closed: Vec<WlOutput>,
    warned: bool,
}

pub(super) struct Surface {
    layer: LayerSurface,
    viewport: WpViewport,
    output: WlOutput,
    entry: Option<usize>,
    label: String,
    config: BackgroundConfig,
    source: Arc<Encoded>,
    upload: Option<Rc<Upload>>,
    fractional: Option<WpFractionalScaleV1>,
    scale: Scale,
    screen: Option<(u32, u32)>,
    pending: bool,
    attached: bool,
    granted: bool,
    previous: Option<Crop>,
    motion: Motion,
    last_frame: Option<Instant>,
    due: Option<Instant>,
    retry_at: Option<Instant>,
    animated: bool,
    failed: bool,
}

impl Wayland {
    pub fn set_background_jobs(&mut self, jobs: Sender<Ready>) {
        self.background.jobs = Some(jobs);
    }

    pub fn set_backgrounds(&mut self, images: Overlays<Encoded>) {
        self.background.images = images;
        let queue = self.queue.clone();
        self.reconcile(&queue, None);
    }

    pub fn render_backgrounds(&mut self, now: Instant, bass: f32) -> bool {
        let frame_time =
            std::time::Duration::from_secs(1) / self.config.visualizer.framerate.max(1);
        for surface in &mut self.background.surfaces {
            if surface.retry_at.is_some_and(|at| at > now) {
                continue;
            }
            surface.retry_at = None;
            let prepared = surface.update_upload(
                &mut self.background.uploads,
                self.background.jobs.as_ref(),
                &self.shm,
                self.format,
                &self.queue,
            );
            if let Err(error) =
                prepared.and_then(|()| surface.render(now, bass, frame_time, &self.queue))
            {
                if !surface.failed {
                    tracing::warn!("could not draw background on {}: {error}", surface.label);
                    surface.failed = true;
                }
                surface.retry_at = Some(now + std::time::Duration::from_secs(1));
            }
        }
        self.background.uploads.prune();
        self.background
            .surfaces
            .iter()
            .any(|surface| surface.animated || surface.pending)
    }

    pub fn has_backgrounds(&self) -> bool {
        !self.background.surfaces.is_empty()
    }
}

impl Backgrounds {
    pub(super) fn clear(&mut self) {
        self.surfaces.clear();
        self.uploads.clear();
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.surfaces
            .iter()
            .filter_map(|surface| surface.retry_at.or(surface.due))
            .min()
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.viewport.destroy();
        if let Some(scale) = self.fractional.take() {
            scale.destroy();
        }
        // the layer role/surface drops before the immutable buffer and pool
    }
}

impl Wayland {
    pub(crate) fn set_followed_backgrounds(
        &mut self,
        images: &[(String, Arc<Encoded>)],
        grants: &[String],
    ) {
        let changed = self.background.external.len() != images.len()
            || self
                .background
                .external
                .iter()
                .zip(images)
                .any(|((a, x), (b, y))| a != b || !Arc::ptr_eq(x, y));
        if changed {
            self.background.external = images.to_vec();
        }
        let grant_changed = self.background.grants != grants;
        if grant_changed {
            self.background.grants = grants.to_vec();
        }
        if changed || grant_changed {
            let queue = self.queue.clone();
            self.reconcile(&queue, None);
        }
    }

    pub(crate) fn follow_selection(&self) -> (u64, &[String]) {
        (self.background.selection, &self.background.following)
    }

    pub(crate) fn ready_backgrounds(&self) -> impl Iterator<Item = (&str, &Arc<Encoded>)> {
        self.background.surfaces.iter().filter_map(|surface| {
            (surface.config.source == BackgroundSource::Sweetbg
                && surface.screen.is_some()
                && surface
                    .upload
                    .as_ref()
                    .is_some_and(|upload| Arc::ptr_eq(&upload.source, &surface.source)))
            .then_some((surface.label.as_str(), &surface.source))
        })
    }
}
