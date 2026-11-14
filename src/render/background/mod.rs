//! bass-response motion and bounded wallpaper cropping, free of Wayland types

#[cfg(test)]
mod tests;

mod motion;
mod sample;
mod size;
pub use motion::{Motion, Response};
pub(crate) use sample::write;
pub use size::Preparation;

/// source coordinates are quantized down to wayland's 24.8 fixed-point grid
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crop(pub f64, pub f64, pub f64, pub f64);

pub fn crop(image: (u32, u32), screen: (u32, u32), zoom: f64, offset: (f64, f64)) -> Option<Crop> {
    if image.0 == 0
        || image.1 == 0
        || screen.0 == 0
        || screen.1 == 0
        || !zoom.is_finite()
        || zoom < 1.0
        || !offset.0.is_finite()
        || !offset.1.is_finite()
    {
        return None;
    }
    let (iw, ih) = (f64::from(image.0), f64::from(image.1));
    let ratio = f64::from(screen.0) / f64::from(screen.1.max(1));
    let width = iw.min(ih * ratio) / zoom;
    let height = ih.min(iw / ratio) / zoom;
    let fixed = |value: f64| (value * 256.0).floor() / 256.0;
    let (width, height) = (
        fixed(width).max(1.0 / 256.0),
        fixed(height).max(1.0 / 256.0),
    );
    let x = ((iw - width) * 0.5 + width * offset.0).clamp(0.0, iw - width);
    let y = ((ih - height) * 0.5 + height * offset.1).clamp(0.0, ih - height);
    Some(Crop(fixed(x), fixed(y), width, height))
}
