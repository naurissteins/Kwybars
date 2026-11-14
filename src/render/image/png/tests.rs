use std::io::Cursor;

use ::png::{BitDepth, ColorType, Encoder};
use image::RgbaImage;

use super::super::{Encoded, ImageError, Source};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn with_text() -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    let mut encoder = Encoder::new(&mut bytes, 2, 1);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    encoder.add_text_chunk("Comment".into(), "unused".repeat(1000))?;
    encoder.add_ztxt_chunk("Description".into(), "unused".repeat(1000))?;
    encoder.add_itxt_chunk("XML:com.adobe.xmp".into(), "metadata".repeat(262_144))?;
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&[10, 20, 30, 255, 50, 100, 200, 128])?;
    writer.write_text_chunk(&::png::text_metadata::TEXtChunk::new("After", "unused"))?;
    writer.write_text_chunk(&::png::text_metadata::ZTXtChunk::new("After", "unused"))?;
    writer.write_text_chunk(&::png::text_metadata::ITXtChunk::new("After", "unused"))?;
    writer.finish()?;
    Ok(bytes)
}

#[test]
fn all_text_types_are_skipped_before_and_after_pixels() -> Result {
    let bytes = with_text()?;
    let mut reader = super::reader(Cursor::new(&bytes))?;
    let mut pixels = [0_u8; 8];
    reader.next_frame(&mut pixels)?;
    reader.finish()?;
    let info = reader.info();
    assert!(info.uncompressed_latin1_text.is_empty());
    assert!(info.compressed_latin1_text.is_empty());
    assert!(info.utf8_text.is_empty());
    let expected = Source::from_rgba(RgbaImage::from_raw(2, 1, pixels.to_vec()).ok_or("pixels")?);
    assert_eq!(Source::decode(Cursor::new(&bytes))?, expected);
    let dir = crate::config::tests::TempDir::new("png-text");
    let file = dir.path().join("text.png");
    std::fs::write(&file, bytes)?;
    assert_eq!(Source::open(&file)?, expected);
    assert_eq!(Encoded::open(&file)?.decode()?, expected);
    Ok(())
}

#[test]
fn packed_palette_and_grayscale_keep_transparency_and_pixel_values() -> Result {
    for color in [ColorType::Indexed, ColorType::Grayscale] {
        let mut bytes = Vec::new();
        let mut encoder = Encoder::new(&mut bytes, 4, 1);
        encoder.set_color(color);
        encoder.set_depth(BitDepth::Two);
        if color == ColorType::Indexed {
            encoder.set_palette(vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 80, 90, 100]);
            encoder.set_trns(vec![255, 128, 0, 255]);
        } else {
            encoder.set_trns(vec![0, 1]);
        }
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&[0b0001_1011])?;
        writer.finish()?;
        let expected = Source::from_rgba(image::load_from_memory(&bytes)?.into_rgba8());
        assert_eq!(Source::decode(Cursor::new(bytes))?, expected);
    }
    Ok(())
}

#[test]
fn oversized_png_is_refused_before_reading_metadata_or_pixels() -> Result {
    let mut bytes = Vec::new();
    let encoder = Encoder::new(&mut bytes, 8000, 5001);
    encoder.write_header()?;
    // The incomplete file has no pixels; rejection must come from its dimensions.
    assert!(matches!(
        Source::decode(Cursor::new(bytes)),
        Err(ImageError::TooLarge {
            width: 8000,
            height: 5001
        })
    ));
    Ok(())
}

#[test]
fn truncated_or_corrupt_pixel_data_is_still_refused() -> Result {
    let mut bytes = with_text()?;
    bytes.truncate(bytes.len() / 2);
    assert!(Source::decode(Cursor::new(bytes)).is_err());
    let mut bytes = Vec::new();
    let mut encoder = Encoder::new(&mut bytes, 1, 1);
    encoder.set_color(ColorType::Rgba);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&[10, 20, 30, 255])?;
    writer.finish()?;
    let Some(at) = bytes.windows(4).position(|chunk| chunk == b"IDAT") else {
        return Err("missing IDAT".into());
    };
    // The first data byte follows this known chunk tag.
    if let Some(byte) = bytes.get_mut(at + 4) {
        *byte ^= 0xff;
    }
    assert!(Source::decode(Cursor::new(bytes)).is_err());
    Ok(())
}
