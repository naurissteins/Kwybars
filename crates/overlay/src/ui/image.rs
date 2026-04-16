use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use gdk_pixbuf::{InterpType, Pixbuf};
use gtk::cairo::Context;
use gtk::gdk::prelude::GdkCairoContextExt;
use kwybars_common::config::{ImageOverlayConfig, ImageOverlayFit};
use tracing::error;

#[derive(Clone)]
pub struct ImageOverlayLayer {
    pixbuf: Pixbuf,
    opacity: f32,
    fit: ImageOverlayFit,
    width: u32,
    height: u32,
    offset_x: f32,
    offset_y: f32,
    cache: Rc<RefCell<Option<ScaledImageCache>>>,
}

#[derive(Clone)]
struct ScaledImageCache {
    key: ImageCacheKey,
    pixbuf: Pixbuf,
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, PartialEq)]
struct ImageCacheKey {
    canvas_width: i32,
    canvas_height: i32,
    draw_width: i32,
    draw_height: i32,
}

#[derive(Clone, Copy)]
struct ImagePlacement {
    key: ImageCacheKey,
    x: f64,
    y: f64,
}

impl ImageOverlayLayer {
    pub fn load(path: &Path, config: &ImageOverlayConfig) -> Result<Self, gtk::glib::Error> {
        let pixbuf = Pixbuf::from_file(path)?;
        Ok(Self {
            pixbuf,
            opacity: config.opacity,
            fit: config.fit,
            width: config.width,
            height: config.height,
            offset_x: config.offset_x,
            offset_y: config.offset_y,
            cache: Rc::new(RefCell::new(None)),
        })
    }

    pub fn draw(&self, ctx: &Context, canvas_width: f64, canvas_height: f64) {
        let Some(placement) = self.placement(canvas_width, canvas_height) else {
            return;
        };

        let image = self.scaled_pixbuf(placement);

        if ctx.save().is_err() {
            error!("kwybars: cairo save failed");
            return;
        }
        ctx.set_source_pixbuf(&image.pixbuf, image.x, image.y);
        if ctx
            .paint_with_alpha(f64::from(self.opacity.clamp(0.0, 1.0)))
            .is_err()
        {
            error!("kwybars: cairo image paint failed");
        }
        if ctx.restore().is_err() {
            error!("kwybars: cairo restore failed");
        }
    }

    fn placement(&self, canvas_width: f64, canvas_height: f64) -> Option<ImagePlacement> {
        if canvas_width <= 0.0 || canvas_height <= 0.0 {
            return None;
        }

        let source_width = f64::from(self.pixbuf.width()).max(1.0);
        let source_height = f64::from(self.pixbuf.height()).max(1.0);
        let target_width = if self.width > 0 {
            f64::from(self.width)
        } else {
            canvas_width
        };
        let target_height = if self.height > 0 {
            f64::from(self.height)
        } else {
            canvas_height
        };

        let (draw_width, draw_height) = match self.fit {
            ImageOverlayFit::Contain => {
                let scale = (target_width / source_width)
                    .min(target_height / source_height)
                    .max(0.0);
                (source_width * scale, source_height * scale)
            }
            ImageOverlayFit::Cover => {
                let scale = (target_width / source_width)
                    .max(target_height / source_height)
                    .max(0.0);
                (source_width * scale, source_height * scale)
            }
            ImageOverlayFit::Stretch => (target_width.max(0.0), target_height.max(0.0)),
        };

        if draw_width <= 0.0 || draw_height <= 0.0 {
            return None;
        }

        let draw_width = rounded_i32(draw_width);
        let draw_height = rounded_i32(draw_height);
        let draw_width_f64 = f64::from(draw_width);
        let draw_height_f64 = f64::from(draw_height);
        let x = ((canvas_width - draw_width_f64) * 0.5) + f64::from(self.offset_x);
        let y = ((canvas_height - draw_height_f64) * 0.5) + f64::from(self.offset_y);

        Some(ImagePlacement {
            key: ImageCacheKey {
                canvas_width: rounded_i32(canvas_width),
                canvas_height: rounded_i32(canvas_height),
                draw_width,
                draw_height,
            },
            x,
            y,
        })
    }

    fn scaled_pixbuf(&self, placement: ImagePlacement) -> ScaledImageCache {
        if let Some(cache) = self.cache.borrow().as_ref()
            && cache.key == placement.key
        {
            return cache.clone();
        }

        let pixbuf = self
            .pixbuf
            .scale_simple(
                placement.key.draw_width,
                placement.key.draw_height,
                InterpType::Bilinear,
            )
            .unwrap_or_else(|| self.pixbuf.clone());
        let next = ScaledImageCache {
            key: placement.key,
            pixbuf,
            x: placement.x,
            y: placement.y,
        };
        self.cache.borrow_mut().replace(next.clone());
        next
    }
}

fn rounded_i32(value: f64) -> i32 {
    value.round().clamp(1.0, f64::from(i32::MAX)) as i32
}
