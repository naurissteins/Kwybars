//! painting bars into pixel buffers, free of wayland types

pub mod canvas;
pub mod damage;
pub mod painter;
pub mod pattern;

pub use canvas::Canvas;
pub use damage::PixelRect;
pub use painter::{BufferContents, Painter};

/// byte order of a pixel in memory
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    Rgba,
    Bgra,
}

/// what every surface should show next
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    pub heights: &'a [f32],
    pub generation: u64,
    pub animating: bool,
}
