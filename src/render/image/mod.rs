//! the image overlay: decoding, placing, and scaling, free of wayland types

mod placement;
mod scale;
#[cfg(test)]
mod tests;

pub use placement::{Child, Layout, Rect, layout};
pub use scale::render;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use image::{ImageReader, Limits, RgbaImage};

use crate::config::ImageOverlayConfig;

/// most pixels an image file may have; a 4k overlay has 8.3 million
pub const MAX_PIXELS: u64 = 40_000_000;

/// an image that cannot be shown
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("{0}")]
    Read(#[from] std::io::Error),
    #[error("{0}")]
    Decode(#[from] image::ImageError),
    #[error("{width}x{height} is over the limit of {} megapixels", MAX_PIXELS / 1_000_000)]
    TooLarge { width: u32, height: u32 },
    #[error("the image is empty")]
    Empty,
}

/// the image to show over the bars and how to place it
#[derive(Debug, Clone, PartialEq)]
pub struct Overlay {
    pub source: Arc<Source>,
    pub config: ImageOverlayConfig,
}

/// the image of every surface, none where no image shows
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Overlays {
    pub base: Option<Overlay>,
    pub outputs: Vec<Option<Overlay>>,
}

impl Overlays {
    /// the image of a surface selected by section `entry`, or by none
    pub fn of(&self, entry: Option<usize>) -> Option<&Overlay> {
        match entry {
            Some(entry) => self.outputs.get(entry)?.as_ref(),
            None => self.base.as_ref(),
        }
    }

    fn all(&self) -> impl Iterator<Item = &Overlay> {
        self.base.iter().chain(self.outputs.iter().flatten())
    }

    /// an image already held with these pixels, so it is not scaled again
    pub fn holding(&self, source: &Arc<Source>) -> Option<&Arc<Source>> {
        self.all()
            .map(|overlay| &overlay.source)
            .find(|held| *held == source)
    }
}

/// a decoded image, premultiplied RGBA
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pixels: RgbaImage,
}

impl Source {
    pub fn open(path: &Path) -> Result<Self, ImageError> {
        let mut reader =
            ImageReader::new(BufReader::new(File::open(path)?)).with_guessed_format()?;
        let (width, height) = reader.into_dimensions()?;
        check_size(width, height)?;
        reader = ImageReader::new(BufReader::new(File::open(path)?)).with_guessed_format()?;
        let mut limits = Limits::default();
        // decoders may need working memory besides the pixels
        limits.max_alloc = Some(MAX_PIXELS * 4 * 2);
        reader.limits(limits);
        Ok(Self::from_rgba(reader.decode()?.into_rgba8()))
    }

    /// premultiplies straight RGBA
    pub fn from_rgba(mut pixels: RgbaImage) -> Self {
        for pixel in pixels.pixels_mut() {
            let alpha = u32::from(pixel.0[3]);
            if alpha < 255 {
                for channel in &mut pixel.0[..3] {
                    *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
                }
            }
        }
        Self { pixels }
    }

    pub fn size(&self) -> (u32, u32) {
        self.pixels.dimensions()
    }
}

fn check_size(width: u32, height: u32) -> Result<(), ImageError> {
    if width == 0 || height == 0 {
        Err(ImageError::Empty)
    } else if u64::from(width) * u64::from(height) > MAX_PIXELS {
        Err(ImageError::TooLarge { width, height })
    } else {
        Ok(())
    }
}
