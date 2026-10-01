use image::imageops::{self, FilterType};

use super::{Layout, Source};
use crate::render::{ByteOrder, PixelRect};

pub fn render(
    source: &Source,
    layout: &Layout,
    area: PixelRect,
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
    let visible = layout.visible;
    for y in visible.y..visible.bottom() {
        for x in visible.x..visible.right() {
            // where this buffer pixel is in the scaled image and in area
            let (Ok(sx), Ok(sy)) = (
                u32::try_from(i64::from(x) - i64::from(layout.origin.0)),
                u32::try_from(i64::from(y) - i64::from(layout.origin.1)),
            ) else {
                continue;
            };
            let (Some(pixel), Some(ax), Some(ay)) = (
                scaled.get_pixel_checked(sx, sy),
                x.checked_sub(area.x),
                y.checked_sub(area.y),
            ) else {
                continue;
            };
            let at = (ay as usize * area.width as usize + ax as usize) * 4;
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
    }
    out
}
