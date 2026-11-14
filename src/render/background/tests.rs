use super::*;
use std::time::Duration;

#[test]
fn response_is_independent_of_frame_rate_and_settles() {
    let simulate = |hz: u32| {
        let mut response = Response::default();
        for _ in 0..hz {
            response.step(0.8, Duration::from_secs_f64(1.0 / f64::from(hz)), 35, 220);
        }
        response.value
    };
    assert!((simulate(30) - simulate(144)).abs() < 0.0001);
    let mut response = Response { value: 1.0 };
    assert!(!response.step(0.0, Duration::from_secs(3), 35, 220));
    assert_eq!(response.value, 0.0);
}

#[test]
fn crops_stay_inside_the_image_at_every_aspect_ratio() {
    for image in [(3840, 2160), (1, 1), (500, 900)] {
        for screen in [(2560, 1440), (1440, 2560), (100, 100)] {
            for zoom in [1.0, 1.04, 1.3] {
                for offset in [(-0.02, -0.02), (0.0, 0.0), (0.02, 0.02)] {
                    let Some(Crop(x, y, w, h)) = crop(image, screen, zoom, offset) else {
                        panic!("valid crop");
                    };
                    assert!(x >= 0.0 && y >= 0.0 && w > 0.0 && h > 0.0);
                    assert!(x + w <= f64::from(image.0) && y + h <= f64::from(image.1));
                }
            }
        }
    }
}

#[test]
fn invalid_crop_inputs_are_rejected_without_panicking() {
    assert_eq!(crop((0, 0), (100, 100), 1.0, (0.0, 0.0)), None);
    assert_eq!(crop((100, 100), (0, 100), 1.0, (0.0, 0.0)), None);
    assert_eq!(crop((100, 100), (100, 100), f64::NAN, (0.0, 0.0)), None);
}

#[test]
fn wallpaper_motion_restarts_smoothly_and_stops_after_silence() {
    use std::time::Instant;
    let config = crate::config::BackgroundConfig::default();
    let mut motion = Motion::default();
    let now = Instant::now();
    assert!(!motion.pose(&config, 0.0, now).animated);
    let wake = motion.pose(&config, 1.0, now + Duration::from_secs(30));
    assert_eq!(wake.zoom, 1.0);
    assert!(wake.animated);
    let moved = motion.pose(
        &config,
        1.0,
        now + Duration::from_secs(30) + Duration::from_millis(16),
    );
    assert!(moved.zoom > 1.0 && moved.zoom <= 1.04 + 1e-6);
    let rest = motion.pose(&config, 0.0, now + Duration::from_secs(35));
    assert!(!rest.animated);
    assert_eq!(rest.zoom, 1.0);
}

#[test]
fn preparation_matches_physical_resolution_and_motion_headroom() -> Result<(), &'static str> {
    use super::Preparation;
    let static_image = Preparation::new((5120, 2880), (1920, 1080), 0.0, 0.0).ok_or("size")?;
    assert_eq!(static_image.size, (1920, 1080));
    assert_eq!((static_image.crop.x, static_image.crop.y), (0, 0));
    let motion = Preparation::new((5120, 2880), (1920, 1080), 0.04, 0.003).ok_or("size")?;
    assert_eq!(motion.size, (2009, 1130));
    // A 2560x1440 logical surface at scale 1.5 uses these physical dimensions.
    let hidpi = Preparation::new((5120, 2880), (3840, 2160), 0.0, 0.0).ok_or("size")?;
    assert_eq!(hidpi.size, (3840, 2160));
    let small = Preparation::new((800, 450), (1920, 1080), 0.25, 0.02).ok_or("size")?;
    assert_eq!(small.size, (800, 450));
    Ok(())
}

#[test]
fn preparation_crops_to_cover_in_portrait_and_rejects_invalid_sizes() -> Result<(), &'static str> {
    use super::Preparation;
    let portrait = Preparation::new((5120, 2880), (1080, 1920), 0.0, 0.0).ok_or("size")?;
    assert_eq!(
        (
            portrait.crop.x,
            portrait.crop.y,
            portrait.crop.width,
            portrait.crop.height
        ),
        (1750, 0, 1620, 2880)
    );
    assert_eq!(portrait.size, (1080, 1920));
    for image in [(0, 20), (20, 0), (8000, 5001)] {
        assert!(Preparation::new(image, (1920, 1080), 0.04, 0.0).is_none());
    }
    assert!(Preparation::new((10, 10), (0, 10), 0.0, 0.0).is_none());
    assert!(Preparation::new((10, 10), (10, 10), f32::NAN, 0.0).is_none());
    assert!(Preparation::new((10, 10), (10, 10), 0.0, -0.1).is_none());
    Ok(())
}

#[test]
fn streamed_preparation_matches_triangle_filter_and_preserves_alpha()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::render::ByteOrder;
    use crate::render::image::{Rect, Source};
    use image::{Rgba, RgbaImage};
    let pixels = RgbaImage::from_fn(31, 17, |x, y| {
        Rgba([
            (x * 8) as u8,
            (y * 15) as u8,
            255,
            if x < 15 { 128 } else { 0 },
        ])
    });
    let source = Source::from_rgba(pixels);
    for size in [(11, 7), (30, 16), (1, 1)] {
        let prep = super::Preparation {
            crop: Rect {
                x: 0,
                y: 0,
                width: 31,
                height: 17,
            },
            size,
        };
        let expected = image::imageops::resize(
            &source,
            size.0,
            size.1,
            image::imageops::FilterType::Triangle,
        );
        let mut rgba = Vec::new();
        super::write(&source, prep, &mut rgba, ByteOrder::Rgba)?;
        for (actual, expected) in rgba.iter().zip(expected.as_raw()) {
            assert!(actual.abs_diff(*expected) <= 1, "{actual} != {expected}");
        }
        let mut bgra = Vec::new();
        super::write(&source, prep, &mut bgra, ByteOrder::Bgra)?;
        for (rgba, bgra) in rgba.as_chunks::<4>().0.iter().zip(bgra.as_chunks::<4>().0) {
            assert_eq!(*bgra, [rgba[2], rgba[1], rgba[0], rgba[3]]);
            assert!(rgba[..3].iter().all(|channel| *channel <= rgba[3]));
        }
    }
    Ok(())
}

#[test]
fn preparation_discards_the_unused_sides_without_stretching()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::render::ByteOrder;
    use crate::render::image::Source;
    use image::{Rgba, RgbaImage};
    let source = Source::from_rgba(RgbaImage::from_fn(12, 4, |x, _| {
        if (4..8).contains(&x) {
            Rgba([0, 255, 0, 255])
        } else {
            Rgba([255, 0, 0, 255])
        }
    }));
    let prep = super::Preparation::new(source.size(), (2, 2), 0.0, 0.0).ok_or("size")?;
    let mut bytes = Vec::new();
    super::write(&source, prep, &mut bytes, ByteOrder::Rgba)?;
    assert_eq!(bytes, [0, 255, 0, 255].repeat(4));
    Ok(())
}
