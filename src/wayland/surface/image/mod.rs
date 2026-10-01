//! the image overlay of one surface: a child surface above the bars, drawn
//! once per image, size, or scale, never per frame

mod layer;

use std::sync::Arc;

use calloop::channel::Sender;
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::reexports::client::QueueHandle;
use smithay_client_toolkit::reexports::client::backend::ObjectId;
use smithay_client_toolkit::reexports::client::protocol::wl_shm;
use smithay_client_toolkit::reexports::protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_v1::WpAlphaModifierV1;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::subcompositor::SubcompositorState;
use tracing::{debug, warn};

use super::OutputSurface;
use crate::config::ImageOverlayConfig;
use crate::render::image::{Child, Overlay, Source, layout, render};
use crate::wayland::Wayland;
use crate::wayland::scale::Scale;
use layer::ImageLayer;

/// protocol objects an image layer is created from
pub struct ImageGlobals<'a> {
    pub compositor: &'a CompositorState,
    pub subcompositor: Option<&'a SubcompositorState>,
    pub viewporter: Option<&'a WpViewporter>,
    pub alpha: Option<&'a WpAlphaModifierV1>,
    pub shm: &'a Shm,
    pub format: wl_shm::Format,
}

/// everything a scaled image depends on
#[derive(Debug, Clone, PartialEq)]
pub struct ImageKey {
    /// counts the images shown since startup
    generation: u64,
    config: ImageOverlayConfig,
    logical: (u32, u32),
    buffer: (u32, u32),
    scale: Scale,
}

/// a worker's scaled image for the surface that asked
pub struct ImageReady {
    surface: ObjectId,
    key: ImageKey,
    image: Option<(Child, Vec<u8>)>,
}

impl ImageReady {
    pub fn surface(&self) -> &ObjectId {
        &self.surface
    }
}

/// the image state of one output surface
#[derive(Default)]
pub struct ImageSlot {
    layer: Option<ImageLayer>,
    shown: Option<ImageKey>,
    asked: Option<ImageKey>,
    failed: bool,
}

impl OutputSurface {
    pub fn update_image(
        &mut self,
        overlay: Option<&(Overlay, u64)>,
        jobs: Option<&Sender<ImageReady>>,
    ) {
        let Some((overlay, generation)) = overlay else {
            self.hide_image();
            return;
        };
        // a hidden surface must not come back with an image that was replaced
        let replaced = |key: &ImageKey| key.generation != *generation;
        if !self.shown && self.image.shown.as_ref().is_some_and(replaced) {
            self.hide_image();
        }
        let (Some((logical, buffer)), Some(jobs), true) = (self.sizes(), jobs, self.shown) else {
            return;
        };
        // compared in place: this runs every frame and must not allocate
        let same = |key: &ImageKey| {
            key.generation == *generation
                && key.config == overlay.config
                && (key.logical, key.buffer, key.scale) == (logical, buffer, self.scale)
        };
        if self.image.shown.as_ref().is_some_and(same)
            || self.image.asked.as_ref().is_some_and(same)
        {
            return;
        }
        let key = ImageKey {
            generation: *generation,
            config: overlay.config.clone(),
            logical,
            buffer,
            scale: self.scale,
        };
        let (source, order) = (Arc::clone(&overlay.source), self.order);
        let (surface, jobs, job) = (self.surface_id(), jobs.clone(), key.clone());
        let worker = std::thread::Builder::new()
            .name("kwybars-image".to_owned())
            .spawn(move || {
                let image = scaled(&source, &job, order);
                // fails only when the main loop is gone
                let _ = jobs.send(ImageReady {
                    surface,
                    key: job,
                    image,
                });
            });
        match worker {
            Ok(_) => self.image.asked = Some(key),
            Err(err) => warn!("could not start scaling the image overlay: {err}"),
        }
    }

    /// shows a worker's image if it is still the one asked for
    pub fn show_image(
        &mut self,
        globals: &ImageGlobals<'_>,
        qh: &QueueHandle<Wayland>,
        ready: ImageReady,
    ) {
        if self.image.asked.as_ref() != Some(&ready.key) {
            return;
        }
        self.image.asked = None;
        let Some((child, pixels)) = ready.image else {
            self.hide_image();
            self.image.shown = Some(ready.key);
            return;
        };
        if self.image.layer.is_none() {
            self.image.layer = ImageLayer::new(globals, qh, self.layer.wl_surface());
        }
        let Some(layer) = self.image.layer.as_mut() else {
            if !self.image.failed {
                warn!("the compositor has no subsurfaces, the image overlay is not shown");
                self.image.failed = true;
            }
            return;
        };
        let shown = layer.show(globals, self.scale, child, pixels);
        match &shown {
            Ok(()) => {
                debug!(
                    "{}: image overlay {}x{} at {}, {}",
                    self.label,
                    child.buffer.width,
                    child.buffer.height,
                    child.buffer.x,
                    child.buffer.y
                );
                self.image.failed = false;
            }
            Err(err) => {
                if !self.image.failed {
                    warn!("could not show the image overlay on {}: {err}", self.label);
                }
                self.image.failed = true;
                self.image.layer = None;
            }
        }
        // shown or not, this image at this size is dealt with: asking
        // again would fail the same way
        self.image.shown = Some(ready.key);
        if self.shown {
            // a child's state takes effect with its parent's commit
            self.layer.commit();
        }
    }

    /// follows the bars' fade; takes effect with the parent's next commit
    pub(super) fn fade_image(&mut self, globals_shm: &Shm, format: wl_shm::Format, opacity: u8) {
        if let Some(layer) = self.image.layer.as_mut()
            && let Err(err) = layer.fade(globals_shm, format, opacity)
            && !self.image.failed
        {
            warn!("could not fade the image overlay on {}: {err}", self.label);
            self.image.failed = true;
        }
    }

    fn hide_image(&mut self) {
        self.image.shown = None;
        self.image.asked = None;
        if let Some(layer) = self.image.layer.take() {
            drop(layer);
            if self.shown {
                self.layer.commit();
            }
        }
    }
}

/// the image placed and scaled for one surface, none when nothing shows
fn scaled(
    source: &Source,
    key: &ImageKey,
    order: crate::render::ByteOrder,
) -> Option<(Child, Vec<u8>)> {
    let placed = layout(
        &key.config,
        source.size(),
        key.logical,
        key.buffer,
        key.scale.factor(),
    )?;
    let child = Child::around(placed.visible, key.logical, key.buffer, key.scale.step());
    let pixels = render(source, &placed, child.buffer, key.config.opacity, order);
    Some((child, pixels))
}
