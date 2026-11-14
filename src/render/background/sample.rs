//! triangle sampling streamed to SHM with scratch space proportional to image width

use std::io::Write;

use image::GenericImageView;

use super::Preparation;
use crate::render::ByteOrder;
use crate::render::image::Source;

struct Weights {
    start: u32,
    end: u32,
    center: f64,
    support: f64,
    total: f32,
}

impl Weights {
    fn new(at: u32, input: u32, output: u32) -> Self {
        let ratio = f64::from(input) / f64::from(output);
        let center = (f64::from(at) + 0.5) * ratio;
        let support = ratio.max(1.0);
        let mut weights = Self {
            start: (center - support).floor().max(0.0) as u32,
            end: ((center + support).ceil() as u32).min(input),
            center,
            support,
            total: 1.0,
        };
        weights.total = weights.iter().map(|(_, weight)| weight).sum();
        weights
    }

    fn iter(&self) -> impl Iterator<Item = (u32, f32)> + '_ {
        (self.start..self.end).map(|pixel| {
            let weight = (1.0 - ((f64::from(pixel) + 0.5 - self.center) / self.support).abs())
                .max(0.0) as f32;
            (pixel, weight / self.total)
        })
    }
}

pub(crate) fn write(
    source: &Source,
    preparation: Preparation,
    writer: &mut impl Write,
    order: ByteOrder,
) -> std::io::Result<()> {
    let Preparation { crop, size } = preparation;
    if (crop.width, crop.height) == source.size() && size == source.size() {
        return source.write_native(writer, order);
    }
    // Avoid full-image float intermediates and packed RGBA copies.
    let mut row = vec![[0.0_f32; 4]; crop.width as usize];
    let mut out = [0_u8; 4096];
    for y in 0..size.1 {
        let vertical = Weights::new(y, crop.height, size.1);
        row.fill([0.0; 4]);
        for (sy, weight) in vertical.iter() {
            for (x, value) in row.iter_mut().enumerate() {
                let pixel = source.get_pixel(crop.x as u32 + x as u32, crop.y as u32 + sy);
                for (sum, channel) in value.iter_mut().zip(pixel.0) {
                    *sum += f32::from(channel) * weight;
                }
            }
        }
        for first in (0..size.0).step_by(out.len() / 4) {
            let count = (size.0 - first).min((out.len() / 4) as u32) as usize;
            // count is bounded by the stack buffer's pixel capacity.
            let bytes = &mut out[..count * 4];
            for (offset, pixel) in bytes.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let weights = Weights::new(first + offset as u32, crop.width, size.0);
                let mut value = [0.0; 4];
                for (sx, weight) in weights.iter() {
                    if let Some(sample) = row.get(sx as usize) {
                        for (sum, channel) in value.iter_mut().zip(sample) {
                            *sum += channel * weight;
                        }
                    }
                }
                *pixel = value.map(|v| v.round().clamp(0.0, 255.0) as u8);
                if order == ByteOrder::Bgra {
                    pixel.swap(0, 2);
                }
            }
            writer.write_all(bytes)?;
        }
    }
    Ok(())
}
