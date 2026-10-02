use image::imageops::{self, FilterType};

use super::{Layout, Rect, Source};
use crate::render::ByteOrder;

/// the scaled image in a buffer the size of `area`, which holds it whole
pub fn render(
    source: &Source,
    layout: &Layout,
    area: Rect,
    opacity: f32,
    order: ByteOrder,
) -> Vec<u8> {
    let scaled = if source.size() == layout.draw {
        None
    } else {
        Some(imageops::resize(
            &source.pixels,
            layout.draw.0,
            layout.draw.1,
            FilterType::Triangle,
        ))
    };
    let scaled = scaled.as_ref().unwrap_or(&source.pixels);
    let share = (opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    let mut out = vec![0_u8; area.width as usize * area.height as usize * 4];
    // where the image starts inside the area
    let (Ok(left), Ok(top)) = (
        usize::try_from(i64::from(layout.origin.0) - i64::from(area.x)),
        usize::try_from(i64::from(layout.origin.1) - i64::from(area.y)),
    ) else {
        return out;
    };
    for (sx, sy, pixel) in scaled.enumerate_pixels() {
        let at = ((top + sy as usize) * area.width as usize + left + sx as usize) * 4;
        let Some(slot) = out.get_mut(at..at + 4) else {
            continue;
        };
        let [r, g, b, a] = pixel
            .0
            .map(|channel| ((u32::from(channel) * share + 127) / 255) as u8);
        slot.copy_from_slice(&match order {
            ByteOrder::Rgba => [r, g, b, a],
            ByteOrder::Bgra => [b, g, r, a],
        });
    }
    out
}
