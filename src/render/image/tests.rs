use image::{Rgba, RgbaImage};

use super::{Child, ImageError, MAX_PIXELS, Source, check_size, layout, render};
use crate::config::{ImageFit, ImageOverlayConfig};
use crate::render::{ByteOrder, PixelRect};

fn config(fit: ImageFit, edit: impl FnOnce(&mut ImageOverlayConfig)) -> ImageOverlayConfig {
    let mut config = ImageOverlayConfig {
        enabled: true,
        fit,
        ..ImageOverlayConfig::default()
    };
    edit(&mut config);
    config
}

fn rect(x: u32, y: u32, width: u32, height: u32) -> PixelRect {
    PixelRect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn contain_fits_the_image_inside_the_surface_and_centers_it() {
    // the user's 500x500 image in a 2520x520 band at 1.5: 520 logical, 780 px
    let placed = layout(
        &config(ImageFit::Contain, |_| {}),
        (500, 500),
        (2520, 520),
        (3780, 780),
        1.5,
    );
    let Some(placed) = placed else {
        panic!("the image shows");
    };
    assert_eq!(placed.draw, (780, 780));
    assert_eq!(placed.origin, (1500, 0));
    assert_eq!(placed.visible, rect(1500, 0, 780, 780));
}

#[test]
fn cover_fills_the_surface_and_is_cropped_to_it() {
    let placed = layout(
        &config(ImageFit::Cover, |_| {}),
        (500, 250),
        (400, 300),
        (400, 300),
        1.0,
    );
    let Some(placed) = placed else {
        panic!("the image shows");
    };
    // scaled by 1.2 to cover the height: 600 wide, 100 off each side
    assert_eq!(placed.draw, (600, 300));
    assert_eq!(placed.origin, (-100, 0));
    assert_eq!(placed.visible, rect(0, 0, 400, 300));
}

#[test]
fn stretch_takes_the_box_and_offsets_move_it() {
    let boxed = config(ImageFit::Stretch, |c| {
        (c.width, c.height) = (100, 40);
        (c.offset_x, c.offset_y) = (30.0, -10.0);
    });
    let Some(placed) = layout(&boxed, (500, 500), (400, 300), (800, 600), 2.0) else {
        panic!("the image shows");
    };
    assert_eq!(placed.draw, (200, 80));
    // centered at 150, 130 logical, then moved
    assert_eq!(placed.origin, (360, 240));
    // a box makes contain and cover fit it instead of the surface
    let contained = config(ImageFit::Contain, |c| (c.width, c.height) = (100, 40));
    let placed = layout(&contained, (500, 500), (400, 300), (400, 300), 1.0);
    assert_eq!(placed.map(|placed| placed.draw), Some((40, 40)));
}

#[test]
fn an_image_moved_off_the_surface_shows_nothing() {
    let away = config(ImageFit::Contain, |c| c.offset_x = 5_000.0);
    assert_eq!(layout(&away, (500, 500), (400, 300), (400, 300), 1.0), None);
    assert_eq!(
        layout(
            &config(ImageFit::Contain, |_| {}),
            (500, 500),
            (0, 300),
            (1, 300),
            1.0
        ),
        None
    );
}

#[test]
fn a_child_lines_up_with_its_parents_pixels() {
    // at 1.5 every 2 logical pixels are 3 buffer pixels
    let child = Child::around(rect(1500, 10, 781, 760), (2520, 520), (3780, 780), (3, 2));
    assert_eq!(child.logical, rect(1000, 6, 522, 508));
    assert_eq!(child.buffer, rect(1500, 9, 783, 762));
    // an odd logical size: the far edge is where the parent's buffer ends
    let child = Child::around(rect(0, 0, 152, 152), (101, 101), (152, 152), (3, 2));
    assert_eq!(child.logical, rect(0, 0, 101, 101));
    assert_eq!(child.buffer, rect(0, 0, 152, 152));
    // whole scales need no rounding
    let child = Child::around(rect(7, 9, 20, 30), (100, 100), (200, 200), (2, 1));
    assert_eq!(child.logical, rect(3, 4, 11, 16));
    assert_eq!(child.buffer, rect(6, 8, 22, 32));
}

#[test]
fn scaling_premultiplies_fades_and_orders_the_bytes() {
    // left half red, right half see-through blue
    let mut pixels = RgbaImage::new(2, 1);
    pixels.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    pixels.put_pixel(1, 0, Rgba([0, 0, 255, 128]));
    let source = Source::from_rgba(pixels);
    let stretched = config(ImageFit::Stretch, |_| {});
    let Some(placed) = layout(&stretched, source.size(), (4, 2), (4, 2), 1.0) else {
        panic!("the image shows");
    };
    let area = rect(0, 0, 4, 2);
    let rgba = render(&source, &placed, area, 1.0, ByteOrder::Rgba);
    assert_eq!(rgba.len(), 4 * 2 * 4);
    assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
    assert_eq!(&rgba[12..16], &[0, 0, 128, 128]);
    let bgra = render(&source, &placed, area, 0.5, ByteOrder::Bgra);
    assert_eq!(&bgra[0..4], &[0, 0, 128, 128]);
    assert_eq!(&bgra[12..16], &[64, 0, 0, 64]);
}

#[test]
fn the_area_around_the_image_stays_clear() {
    let source = Source::from_rgba(RgbaImage::from_pixel(2, 2, Rgba([0, 255, 0, 255])));
    let small = config(ImageFit::Stretch, |c| (c.width, c.height) = (2, 2));
    let Some(placed) = layout(&small, source.size(), (6, 4), (6, 4), 1.0) else {
        panic!("the image shows");
    };
    assert_eq!(placed.visible, rect(2, 1, 2, 2));
    // an area one pixel larger on every side
    let area = rect(1, 0, 4, 4);
    let out = render(&source, &placed, area, 1.0, ByteOrder::Rgba);
    let pixel = |x: usize, y: usize| &out[(y * 4 + x) * 4..(y * 4 + x) * 4 + 4];
    assert_eq!(pixel(0, 0), &[0; 4]);
    assert_eq!(pixel(1, 1), &[0, 255, 0, 255]);
    assert_eq!(pixel(2, 2), &[0, 255, 0, 255]);
    assert_eq!(pixel(3, 3), &[0; 4]);
}

#[test]
fn files_are_decoded_and_oversized_ones_refused() {
    let dir = std::env::temp_dir().join(format!("kwybars-image-test-{}", std::process::id()));
    assert!(std::fs::create_dir_all(&dir).is_ok());
    let path = dir.join("small.png");
    let saved = RgbaImage::from_pixel(3, 2, Rgba([10, 20, 30, 255])).save(&path);
    assert!(saved.is_ok(), "{saved:?}");
    let opened = Source::open(&path);
    assert_eq!(opened.as_ref().map(Source::size).ok(), Some((3, 2)));
    let broken = dir.join("broken.png");
    assert!(std::fs::write(&broken, b"not an image").is_ok());
    assert!(Source::open(&broken).is_err());
    assert!(matches!(
        Source::open(&dir.join("missing.png")),
        Err(ImageError::Read(_))
    ));
    let _ = std::fs::remove_dir_all(&dir);

    assert!(check_size(8_000, 5_000).is_ok());
    let refused = check_size(8_000, 5_001);
    assert!(
        matches!(refused, Err(ImageError::TooLarge { .. })),
        "{refused:?}"
    );
    assert_eq!(
        refused.err().map(|err| err.to_string()),
        Some(format!(
            "8000x5001 is over the limit of {} megapixels",
            MAX_PIXELS / 1_000_000
        ))
    );
    assert!(matches!(check_size(0, 10), Err(ImageError::Empty)));
}
