//! translucent placeholder fill that proves placement until bars are drawn

use smithay_client_toolkit::reexports::client::protocol::wl_shm;
use smithay_client_toolkit::shm::slot::{ActivateSlotError, Buffer, CreateBufferError, SlotPool};
use smithay_client_toolkit::shm::{CreatePoolError, Shm};

use crate::config::{Rgba, SurfaceConfig};

/// opacity applied on top of the surface color
const OPACITY: f32 = 0.35;

/// a buffer could not be drawn or shown
#[derive(Debug, thiserror::Error)]
pub enum DrawError {
    #[error("surface too large")]
    TooLarge,
    #[error("shared memory pool: {0}")]
    Pool(#[from] CreatePoolError),
    #[error("shared memory buffer: {0}")]
    Buffer(#[from] CreateBufferError),
    #[error("attaching the buffer: {0}")]
    Attach(#[from] ActivateSlotError),
}

/// the first theme color, else the configured bar color, made translucent
pub fn color(config: &SurfaceConfig) -> Rgba {
    let color = config
        .theme_colors
        .map_or(config.visualizer.color_rgba, |colors| colors[0]);
    Rgba {
        a: color.a * OPACITY,
        ..color
    }
}

/// a `width` x `height` buffer filled with `color`, the pool is created on
/// first use and grows with the surface
pub fn buffer(
    pool: &mut Option<SlotPool>,
    shm: &Shm,
    (width, height): (u32, u32),
    color: Rgba,
) -> Result<Buffer, DrawError> {
    let (Ok(w), Ok(h)) = (i32::try_from(width), i32::try_from(height)) else {
        return Err(DrawError::TooLarge);
    };
    let stride = w.checked_mul(4).ok_or(DrawError::TooLarge)?;
    let len = usize::try_from(stride)
        .ok()
        .and_then(|stride| stride.checked_mul(usize::try_from(h).ok()?))
        .ok_or(DrawError::TooLarge)?;
    let pool = match pool {
        Some(pool) => pool,
        None => pool.insert(SlotPool::new(len, shm)?),
    };
    let (buffer, canvas) = pool.create_buffer(w, h, stride, wl_shm::Format::Argb8888)?;
    canvas
        .as_chunks_mut::<4>()
        .0
        .fill(premultiplied_argb(color));
    Ok(buffer)
}

/// `Argb8888` is little-endian, so the bytes are b, g, r, a
fn premultiplied_argb(color: Rgba) -> [u8; 4] {
    let alpha = color.a.clamp(0.0, 1.0);
    let channel = |value: f32| (value.clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
    [
        channel(color.b),
        channel(color.g),
        channel(color.r),
        (alpha * 255.0).round() as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::premultiplied_argb;
    use crate::config::Rgba;

    #[test]
    fn colors_are_premultiplied_in_argb_byte_order() {
        assert_eq!(
            premultiplied_argb(Rgba::new(1.0, 0.5, 0.0, 0.5)),
            [0, 64, 128, 128]
        );
        assert_eq!(premultiplied_argb(Rgba::new(1.0, 1.0, 1.0, 0.0)), [0; 4]);
    }
}
