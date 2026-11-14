//! image sources, placement, and scaling, free of wayland types

mod encoded;
mod placement;
mod scale;
mod source;
#[cfg(test)]
mod tests;

pub use encoded::Encoded;
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
    #[error("encoded image must be between 1 and 320000000 bytes")]
    EncodedSize,
    #[error("the image is empty")]
    Empty,
}

/// the image to show over the bars and how to place it
#[derive(Debug, PartialEq)]
pub struct Overlay<T = Source> {
    pub source: Arc<T>,
    pub config: ImageOverlayConfig,
}

/// the image of every surface, none where no image shows
#[derive(Debug, PartialEq)]
pub struct Overlays<T = Source> {
    pub base: Option<Overlay<T>>,
    pub outputs: Vec<Option<Overlay<T>>>,
}

impl<T> Clone for Overlay<T> {
    fn clone(&self) -> Self {
        Self {
            source: Arc::clone(&self.source),
            config: self.config.clone(),
        }
    }
}

impl<T> Clone for Overlays<T> {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            outputs: self.outputs.clone(),
        }
    }
}

impl<T> Default for Overlays<T> {
    fn default() -> Self {
        Self {
            base: None,
            outputs: Vec::new(),
        }
    }
}

impl<T> Overlays<T> {
    /// the image of a surface selected by section `entry`, or by none
    pub fn of(&self, entry: Option<usize>) -> Option<&Overlay<T>> {
        match entry {
            Some(entry) => self.outputs.get(entry)?.as_ref(),
            None => self.base.as_ref(),
        }
    }
}

impl Overlays {
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
