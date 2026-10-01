//! the image overlay's state and its workers' results

use std::sync::Arc;

use calloop::LoopHandle;
use calloop::channel;

use super::App;
use crate::config::{Config, LoadedImage};
use crate::render::image::Overlay;

/// lets image workers reach the surfaces they scale for
pub(super) fn start(
    handle: &LoopHandle<'static, App>,
    app: &mut App,
) -> Result<(), calloop::Error> {
    let (jobs, results) = channel::channel();
    handle
        .insert_source(results, |event, (), app| {
            if let channel::Event::Msg(ready) = event {
                app.wayland.image_ready(ready);
            }
        })
        .map_err(|err| err.error)?;
    app.wayland.set_image_jobs(jobs);
    app.wayland.set_image(app.image.clone());
    Ok(())
}

pub(super) fn overlay(
    config: &Config,
    loaded: Option<LoadedImage>,
    showing: Option<&Overlay>,
) -> Option<Overlay> {
    let source = match loaded?.source {
        Ok(source) => match showing {
            Some(showing) if showing.source == source => Arc::clone(&showing.source),
            _ => source,
        },
        Err(_) => Arc::clone(&showing?.source),
    };
    Some(Overlay {
        source,
        config: config.image_overlay.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use image::{Rgba, RgbaImage};

    use super::overlay;
    use crate::config::{Config, LoadedImage};
    use crate::render::image::Source;

    fn loaded(shade: Option<u8>) -> Option<LoadedImage> {
        Some(LoadedImage {
            path: PathBuf::from("/overlays/a.png"),
            source: match shade {
                Some(shade) => {
                    let pixels = RgbaImage::from_pixel(2, 2, Rgba([shade, 0, 0, 255]));
                    Ok(Arc::new(Source::from_rgba(pixels)))
                }
                None => Err("broken".to_owned()),
            },
        })
    }

    #[test]
    fn a_broken_image_keeps_the_one_showing() {
        let config = Config::default();
        assert_eq!(overlay(&config, None, None), None);
        assert_eq!(overlay(&config, loaded(None), None), None);
        let Some(first) = overlay(&config, loaded(Some(7)), None) else {
            panic!("an image that loads is shown");
        };
        // the same pixels again: the very same image, so nothing is rescaled
        let again = overlay(&config, loaded(Some(7)), Some(&first));
        assert!(again.is_some_and(|again| Arc::ptr_eq(&again.source, &first.source)));
        let other = overlay(&config, loaded(Some(8)), Some(&first));
        assert!(other.is_some_and(|other| other.source != first.source));
        // broken: the old pixels with the new settings
        let mut moved = config.clone();
        moved.image_overlay.offset_x = 12.0;
        let kept = overlay(&moved, loaded(None), Some(&first));
        assert!(kept.is_some_and(|kept| {
            Arc::ptr_eq(&kept.source, &first.source) && kept.config.offset_x == 12.0
        }));
        // turned off or without a path: nothing
        assert_eq!(overlay(&config, None, Some(&first)), None);
    }
}
