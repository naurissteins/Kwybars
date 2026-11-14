//! the image overlay: decoding, placing, and scaling, free of wayland types

mod placement;
mod scale;
mod source;
#[cfg(test)]
mod tests;

pub use placement::{Child, Layout, Rect, layout};
pub use scale::render;
pub use source::Source;

use std::sync::Arc;

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
    #[error("unsupported decoded color type {0:?}")]
    Color(image::ColorType),
    #[error("decoded image dimensions do not match pixel storage")]
    InvalidPixels,
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

fn check_size(width: u32, height: u32) -> Result<(), ImageError> {
    if width == 0 || height == 0 {
        Err(ImageError::Empty)
    } else if u64::from(width) * u64::from(height) > MAX_PIXELS {
        Err(ImageError::TooLarge { width, height })
    } else {
        Ok(())
    }
}
