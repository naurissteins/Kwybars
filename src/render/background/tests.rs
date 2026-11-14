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
