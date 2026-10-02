use smithay_client_toolkit::compositor::Region;
use smithay_client_toolkit::reexports::client::QueueHandle;
use smithay_client_toolkit::reexports::client::protocol::{
    wl_shm, wl_subsurface::WlSubsurface, wl_surface::WlSurface,
};
use smithay_client_toolkit::reexports::protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};

use super::ImageGlobals;
use crate::render::image::Child;
use crate::wayland::Wayland;
use crate::wayland::handlers::NoEvents;
use crate::wayland::scale::Scale;
use crate::wayland::surface::buffers::DrawError;

/// the child surface and its one buffer
pub struct ImageLayer {
    surface: WlSurface,
    subsurface: WlSubsurface,
    viewport: Option<WpViewport>,
    alpha: Option<WpAlphaModifierSurfaceV1>,
    pool: Option<SlotPool>,
    buffer: Option<Buffer>,
    pixels: Option<(Child, Vec<u8>)>,
    opacity: u8,
}

impl ImageLayer {
    /// a click-through child above parent; none without wl_subcompositor
    pub fn new(
        globals: &ImageGlobals<'_>,
        qh: &QueueHandle<Wayland>,
        parent: &WlSurface,
    ) -> Option<Self> {
        let (subsurface, surface) = globals.subcompositor?.create_subsurface(parent.clone(), qh);
        if let Ok(region) = Region::new(globals.compositor) {
            surface.set_input_region(Some(region.wl_region()));
        }
        Some(Self {
            viewport: globals
                .viewporter
                .map(|viewporter| viewporter.get_viewport(&surface, qh, NoEvents)),
            alpha: globals
                .alpha
                .map(|alpha| alpha.get_surface(&surface, qh, NoEvents)),
            surface,
            subsurface,
            pool: None,
            buffer: None,
            pixels: None,
            opacity: u8::MAX,
        })
    }

    /// puts pixels, a buffer of child's size, at child's place
    pub fn show(
        &mut self,
        globals: &ImageGlobals<'_>,
        scale: Scale,
        child: Child,
        pixels: Vec<u8>,
    ) -> Result<(), DrawError> {
        let int = |value: u32| i32::try_from(value).map_err(|_| DrawError::TooLarge);
        // the image may start left of or above the bars' surface
        self.subsurface
            .set_position(child.logical.x, child.logical.y);
        match (&self.viewport, scale) {
            (Some(viewport), _) => {
                self.surface.set_buffer_scale(1);
                viewport.set_destination(int(child.logical.width)?, int(child.logical.height)?);
            }
            (None, Scale::Integer(scale)) => self.surface.set_buffer_scale(int(scale)?),
            (None, Scale::Fractional(_)) => self.surface.set_buffer_scale(1),
        }
        self.pixels = Some((child, pixels));
        self.attach(globals.shm, globals.format)
    }

    /// a new opacity: the compositor's multiplier when it has one, else the
    /// pixels are written again
    pub fn fade(
        &mut self,
        shm: &Shm,
        format: wl_shm::Format,
        opacity: u8,
    ) -> Result<(), DrawError> {
        if opacity == self.opacity {
            return Ok(());
        }
        self.opacity = opacity;
        match &self.alpha {
            Some(alpha) => {
                alpha.set_multiplier(u32::from(opacity) * (u32::MAX / 255));
                self.surface.commit();
                Ok(())
            }
            None => self.attach(shm, format),
        }
    }

    /// writes the pixels into a new buffer and attaches it
    fn attach(&mut self, shm: &Shm, format: wl_shm::Format) -> Result<(), DrawError> {
        let Some((child, pixels)) = &self.pixels else {
            return Ok(());
        };
        let int = |value: u32| i32::try_from(value).map_err(|_| DrawError::TooLarge);
        let (width, height) = (int(child.buffer.width)?, int(child.buffer.height)?);
        let stride = width.checked_mul(4).ok_or(DrawError::TooLarge)?;
        if self.pool.is_none() {
            self.pool = Some(SlotPool::new(pixels.len().max(4), shm)?);
        }
        let Some(pool) = self.pool.as_mut() else {
            return Ok(());
        };
        let (buffer, canvas) = pool.create_buffer(width, height, stride, format)?;
        // a pool's slots are rounded up, so the canvas may be longer
        let Some(canvas) = canvas.get_mut(..pixels.len()) else {
            return Err(DrawError::TooLarge);
        };
        match (&self.alpha, self.opacity) {
            (Some(alpha), opacity) => {
                canvas.copy_from_slice(pixels);
                alpha.set_multiplier(u32::from(opacity) * (u32::MAX / 255));
            }
            (None, u8::MAX) => canvas.copy_from_slice(pixels),
            (None, opacity) => {
                for (out, value) in canvas.iter_mut().zip(pixels) {
                    *out = ((u16::from(*value) * u16::from(opacity) + 127) / 255) as u8;
                }
            }
        }
        buffer.attach_to(&self.surface)?;
        self.surface.damage_buffer(0, 0, width, height);
        self.surface.commit();
        self.buffer = Some(buffer);
        if self.alpha.is_some() {
            self.pixels = None;
        }
        Ok(())
    }
}

impl Drop for ImageLayer {
    fn drop(&mut self) {
        if let Some(alpha) = self.alpha.take() {
            alpha.destroy();
        }
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
        self.subsurface.destroy();
        self.surface.destroy();
    }
}
