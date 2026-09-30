//! painting bars into pixel buffers, free of wayland types

pub mod canvas;
pub mod damage;
pub mod dots;
pub mod fill;
pub mod frame;
pub mod geometry;
pub mod line;
pub mod mirror;
pub mod painter;
pub mod raster;

pub use canvas::Canvas;
pub use damage::PixelRect;
pub use painter::{BufferContents, Painter};

use std::time::Instant;

use crate::config::Rgba;

/// byte order of a pixel in memory
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    Rgba,
    Bgra,
}

impl ByteOrder {
    /// `color` premultiplied by its alpha, as bytes in this order
    pub fn pack(self, color: Rgba) -> [u8; 4] {
        let alpha = color.a.clamp(0.0, 1.0);
        let channel = |value: f32| (value.clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
        let (r, g, b) = (channel(color.r), channel(color.g), channel(color.b));
        let a = (alpha * 255.0).round() as u8;
        match self {
            Self::Rgba => [r, g, b, a],
            Self::Bgra => [b, g, r, a],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pose {
    pub extent: f32,
    pub shift: f32,
}

/// what every surface should show next
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    pub heights: &'a [f32],
    pub generation: u64,
    pub time: Instant,
    pub animating: bool,
}

#[cfg(test)]
mod tests {
    use super::ByteOrder;
    use crate::config::Rgba;

    #[test]
    fn packs_premultiplied_in_either_order() {
        let orange = Rgba::new(1.0, 0.5, 0.0, 0.5);
        assert_eq!(ByteOrder::Rgba.pack(orange), [128, 64, 0, 128]);
        assert_eq!(ByteOrder::Bgra.pack(orange), [0, 64, 128, 128]);
        assert_eq!(ByteOrder::Rgba.pack(Rgba::new(1.0, 1.0, 1.0, 0.0)), [0; 4]);
    }
}
