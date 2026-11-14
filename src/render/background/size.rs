//! cover crop and physical resolution needed at maximum motion

use crate::render::image::{MAX_PIXELS, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preparation {
    pub crop: Rect,
    pub size: (u32, u32),
}

impl Preparation {
    pub fn new(image: (u32, u32), physical: (u32, u32), zoom: f32, shake: f32) -> Option<Self> {
        if image.0 == 0
            || image.1 == 0
            || physical.0 == 0
            || physical.1 == 0
            || !zoom.is_finite()
            || !shake.is_finite()
            || zoom < 0.0
            || shake < 0.0
            || u64::from(image.0) * u64::from(image.1) > MAX_PIXELS
        {
            return None;
        }
        let ratio = f64::from(physical.0) / f64::from(physical.1);
        let width = f64::from(image.0)
            .min(f64::from(image.1) * ratio)
            .floor()
            .max(1.0) as u32;
        let height = f64::from(image.1)
            .min(f64::from(image.0) / ratio)
            .floor()
            .max(1.0) as u32;
        let headroom = 1.0 + f64::from(zoom) + 2.0 * f64::from(shake);
        let scale = (f64::from(physical.0) * headroom / f64::from(width))
            .max(f64::from(physical.1) * headroom / f64::from(height))
            .min(1.0);
        Some(Self {
            crop: Rect {
                x: ((image.0 - width) / 2) as i32,
                y: ((image.1 - height) / 2) as i32,
                width,
                height,
            },
            size: (
                (f64::from(width) * scale).ceil() as u32,
                (f64::from(height) * scale).ceil() as u32,
            ),
        })
    }
}
