//! PNG pixels without unused text metadata, preserving palette/alpha/16-bit expansion

#[cfg(test)]
mod tests;

use std::io::{BufRead, Seek};

use ::png::{BitDepth, ColorType, Decoder, Limits, Reader, Transformations};
use memmap2::MmapMut;

use super::{ImageError, MAX_PIXELS, Source, check_size};

pub(super) fn decode(input: impl BufRead + Seek) -> Result<Source, ImageError> {
    let mut reader = reader(input)?;
    let size = reader.info().size();
    let (color, depth) = reader.output_color_type();
    let color = color_type(color, depth)?;
    let len = reader
        .output_buffer_size()
        .ok_or(ImageError::InvalidPixels)?;
    if len as u64 > MAX_PIXELS * 8 {
        return Err(ImageError::InvalidPixels);
    }
    let mut pixels = MmapMut::map_anon(len)?;
    reader.next_frame(&mut pixels)?;
    if depth == BitDepth::Sixteen {
        // Match image's native-endian ImageDecoder contract before RGBA conversion.
        for pair in pixels.as_chunks_mut::<2>().0 {
            *pair = u16::from_be_bytes(*pair).to_ne_bytes();
        }
    }
    Source::from_decoded(size, color, pixels)
}

fn reader<R: BufRead + Seek>(input: R) -> Result<Reader<R>, ImageError> {
    let limits = Limits {
        bytes: (MAX_PIXELS * 8) as usize,
    };
    let mut decoder = Decoder::new_with_limits(input, limits);
    // Text/XMP is never used for drawing; large text chunks otherwise leave heap arenas resident.
    decoder.set_ignore_text_chunk(true);
    let header = decoder.read_header_info()?;
    check_size(header.width, header.height)?;
    decoder.set_transformations(Transformations::EXPAND);
    Ok(decoder.read_info()?)
}

fn color_type(color: ColorType, depth: BitDepth) -> Result<image::ColorType, ImageError> {
    use image::ColorType as ImageColor;
    match (color, depth) {
        (ColorType::Grayscale, BitDepth::Eight) => Ok(ImageColor::L8),
        (ColorType::GrayscaleAlpha, BitDepth::Eight) => Ok(ImageColor::La8),
        (ColorType::Rgb, BitDepth::Eight) => Ok(ImageColor::Rgb8),
        (ColorType::Rgba, BitDepth::Eight) => Ok(ImageColor::Rgba8),
        (ColorType::Grayscale, BitDepth::Sixteen) => Ok(ImageColor::L16),
        (ColorType::GrayscaleAlpha, BitDepth::Sixteen) => Ok(ImageColor::La16),
        (ColorType::Rgb, BitDepth::Sixteen) => Ok(ImageColor::Rgb16),
        (ColorType::Rgba, BitDepth::Sixteen) => Ok(ImageColor::Rgba16),
        // EXPAND removes packed and indexed output formats.
        _ => Err(ImageError::InvalidPixels),
    }
}
