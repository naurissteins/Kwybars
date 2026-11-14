//! decoded pixels live outside the allocator's retained arenas

use std::fs::File;
use std::io::{BufRead, BufReader, Seek};
use std::path::Path;

use image::{
    ColorType, GenericImageView, ImageBuffer, ImageDecoder, ImageFormat, ImageReader, Limits, Rgba,
    RgbaImage,
};
use memmap2::MmapMut;

use super::{ImageError, MAX_PIXELS, check_size};
use crate::render::ByteOrder;

#[derive(Debug)]
enum Pixels {
    Heap(RgbaImage),
    Mapped(ImageBuffer<Rgba<u8>, MmapMut>),
}

/// a decoded, premultiplied RGBA image; mapped pixels are released on drop
#[derive(Debug)]
pub struct Source {
    pixels: Pixels,
}

impl Source {
    pub fn open(path: &Path) -> Result<Self, ImageError> {
        Self::decode(BufReader::new(File::open(path)?))
    }

    pub(super) fn decode(reader: impl BufRead + Seek) -> Result<Self, ImageError> {
        let mut reader = ImageReader::new(reader).with_guessed_format()?;
        if reader.format() == Some(ImageFormat::Png) {
            return super::png::decode(reader.into_inner());
        }
        let mut limits = Limits::default();
        limits.max_alloc = Some(MAX_PIXELS * 4 * 2);
        reader.limits(limits);
        let decoder = reader.into_decoder()?;
        let (width, height) = decoder.dimensions();
        check_size(width, height)?;
        let color = decoder.color_type();
        let len = usize::try_from(decoder.total_bytes()).map_err(|_| ImageError::InvalidPixels)?;
        if len as u64 > MAX_PIXELS * 8 {
            return Err(ImageError::InvalidPixels);
        }
        let mut decoded = MmapMut::map_anon(len)?;
        decoder.read_image(&mut decoded)?;
        Self::from_decoded((width, height), color, decoded)
    }

    pub(super) fn from_decoded(
        size: (u32, u32),
        color: ColorType,
        decoded: MmapMut,
    ) -> Result<Self, ImageError> {
        let (width, height) = size;
        let pixels = if color == ColorType::Rgba8 {
            decoded
        } else {
            let mut rgba = MmapMut::map_anon(width as usize * height as usize * 4)?;
            expand(color, &decoded, &mut rgba)?;
            rgba
        };
        let mut pixels =
            ImageBuffer::from_raw(width, height, pixels).ok_or(ImageError::InvalidPixels)?;
        premultiply(pixels.as_mut());
        Ok(Self {
            pixels: Pixels::Mapped(pixels),
        })
    }

    /// premultiplies straight RGBA supplied by in-process callers
    pub fn from_rgba(mut pixels: RgbaImage) -> Self {
        premultiply(pixels.as_mut());
        Self {
            pixels: Pixels::Heap(pixels),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        match &self.pixels {
            Pixels::Heap(pixels) => pixels.dimensions(),
            Pixels::Mapped(pixels) => pixels.dimensions(),
        }
    }

    fn bytes(&self) -> &[u8] {
        match &self.pixels {
            Pixels::Heap(pixels) => pixels.as_raw(),
            Pixels::Mapped(pixels) => pixels.as_raw(),
        }
    }

    /// streams native pixels without allocating a second image-sized buffer
    pub(crate) fn write_native(
        &self,
        writer: &mut impl std::io::Write,
        order: ByteOrder,
    ) -> std::io::Result<()> {
        let pixels = self.bytes();
        if order == ByteOrder::Rgba {
            return writer.write_all(pixels);
        }
        let mut converted = [0_u8; 16_384];
        for chunk in pixels.chunks(converted.len()) {
            // chunks are at most the scratch length and contain whole RGBA pixels
            let out = &mut converted[..chunk.len()];
            out.copy_from_slice(chunk);
            for pixel in out.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
            writer.write_all(out)?;
        }
        Ok(())
    }
}

impl GenericImageView for Source {
    type Pixel = Rgba<u8>;
    fn dimensions(&self) -> (u32, u32) {
        self.size()
    }
    fn get_pixel(&self, x: u32, y: u32) -> Rgba<u8> {
        // GenericImageView callers supply in-bounds coordinates, as for ImageBuffer.
        match &self.pixels {
            Pixels::Heap(pixels) => *pixels.get_pixel(x, y),
            Pixels::Mapped(pixels) => *pixels.get_pixel(x, y),
        }
    }
}

impl PartialEq for Source {
    fn eq(&self, other: &Self) -> bool {
        self.size() == other.size() && self.bytes() == other.bytes()
    }
}
impl Eq for Source {}

fn premultiply(pixels: &mut [u8]) {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        if alpha < 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
            }
        }
    }
}

fn expand(color: ColorType, decoded: &[u8], rgba: &mut [u8]) -> Result<(), ImageError> {
    let (channels, sixteen) = match color {
        ColorType::L8 => (1, false),
        ColorType::La8 => (2, false),
        ColorType::Rgb8 => (3, false),
        ColorType::L16 => (1, true),
        ColorType::La16 => (2, true),
        ColorType::Rgb16 => (3, true),
        ColorType::Rgba16 => (4, true),
        _ => return Err(ImageError::Color(color)),
    };
    let bytes = channels * if sixteen { 2 } else { 1 };
    if decoded.len() / bytes != rgba.len() / 4 {
        return Err(ImageError::InvalidPixels);
    }
    for (input, output) in decoded.chunks_exact(bytes).zip(rgba.as_chunks_mut::<4>().0) {
        let mut components = [0, 0, 0, 255];
        if sixteen {
            for (component, pair) in components.iter_mut().zip(input.as_chunks::<2>().0) {
                // Match image's u16 -> u8 rounding exactly.
                *component = ((u32::from(u16::from_ne_bytes(*pair)) + 128) / 257) as u8;
            }
        } else {
            components[..channels].copy_from_slice(input);
        }
        *output = match channels {
            1 => [components[0], components[0], components[0], 255],
            2 => [components[0], components[0], components[0], components[1]],
            _ => components,
        };
    }
    Ok(())
}
