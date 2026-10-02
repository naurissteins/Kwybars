//! the image overlay's state and its workers' results

use std::sync::Arc;

use calloop::LoopHandle;
use calloop::channel;

use super::App;
use crate::config::{ImageOverlayConfig, Loaded, LoadedImage};
use crate::render::image::{Overlay, Overlays};

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
    app.wayland.set_images(app.images.clone());
    Ok(())
}

/// the images to show after a load; a file that could not be read keeps
/// the pixels its surface is showing, with the new settings
pub(super) fn overlays(loaded: &Loaded, showing: &Overlays) -> Overlays {
    let config = &loaded.config;
    Overlays {
        base: overlay(
            config.image(None),
            loaded.image.as_ref(),
            showing.base.as_ref(),
            showing,
        ),
        outputs: config
            .overlay
            .outputs
            .iter()
            .enumerate()
            .map(|(entry, output)| {
                overlay(
                    config.image(Some(output)),
                    loaded.output_images.get(entry).and_then(Option::as_ref),
                    showing.of(Some(entry)),
                    showing,
                )
            })
            .collect(),
    }
}

fn overlay(
    config: ImageOverlayConfig,
    loaded: Option<&LoadedImage>,
    previous: Option<&Overlay>,
    showing: &Overlays,
) -> Option<Overlay> {
    let source = match &loaded?.source {
        Ok(source) => Arc::clone(showing.holding(source).unwrap_or(source)),
        Err(_) => Arc::clone(&previous?.source),
    };
    Some(Overlay { source, config })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use image::{Rgba, RgbaImage};

    use super::overlays;
    use crate::config::tests::TempDir;
    use crate::config::{self, Loaded};
    use crate::render::image::Overlays;
    use crate::xdg::fake_env;

    fn save(dir: &TempDir, name: &str, shade: u8) {
        let pixels = RgbaImage::from_pixel(2, 2, Rgba([shade, 0, 0, 255]));
        let saved = pixels.save(dir.path().join(name));
        assert!(saved.is_ok(), "{saved:?}");
    }

    fn load(dir: &TempDir, raw: &str) -> Loaded {
        let path = dir.write("config.toml", raw);
        match config::load(&path, &fake_env(&[])) {
            Ok(loaded) => loaded,
            Err(err) => panic!("{err}"),
        }
    }

    const ONE: &str = "[image_overlay]\nenabled = true\npath = \"a.png\"\n";

    #[test]
    fn a_broken_image_keeps_the_one_showing() {
        let dir = TempDir::new("app-image");
        let none = Overlays::default();
        assert_eq!(overlays(&load(&dir, ""), &none), none);
        // a file that cannot be read shows nothing at first
        dir.write("a.png", "not an image");
        assert_eq!(overlays(&load(&dir, ONE), &none).base, None);

        save(&dir, "a.png", 7);
        let first = overlays(&load(&dir, ONE), &none);
        let Some(shown) = &first.base else {
            panic!("an image that loads is shown");
        };
        // the same pixels again: the very same image, so nothing is rescaled
        let again = overlays(&load(&dir, ONE), &first);
        assert!(
            again
                .base
                .is_some_and(|again| Arc::ptr_eq(&again.source, &shown.source))
        );
        save(&dir, "a.png", 8);
        let other = overlays(&load(&dir, ONE), &first);
        assert!(other.base.is_some_and(|other| other.source != shown.source));
        // broken: the old pixels with the new settings
        dir.write("a.png", "not an image");
        let kept = overlays(&load(&dir, &format!("{ONE}offset_x = 12\n")), &first);
        assert!(kept.base.is_some_and(|kept| {
            Arc::ptr_eq(&kept.source, &shown.source) && kept.config.offset_x == 12.0
        }));
        // turned off: nothing
        assert_eq!(overlays(&load(&dir, ""), &first), none);
    }

    #[test]
    fn each_output_section_can_have_its_own_image() {
        let dir = TempDir::new("app-images");
        save(&dir, "a.png", 1);
        save(&dir, "b.png", 2);
        let loaded = load(
            &dir,
            &format!(
                "{ONE}[output.DP-1.image_overlay]\npath = \"b.png\"\nopacity = 0.5\n\
                 [output.DP-2]\nheight = 100\n\
                 [output.DP-3.image_overlay]\nenabled = false\n"
            ),
        );
        let shown = overlays(&loaded, &Overlays::default());
        let Some(base) = &shown.base else {
            panic!("the base image shows");
        };
        let [Some(own), Some(same), None] = shown.outputs.as_slice() else {
            panic!("expected two images and none, got {:?}", shown.outputs);
        };
        assert!(own.source != base.source);
        assert_eq!((own.config.opacity, base.config.opacity), (0.5, 1.0));
        // a section without image keys shows the base image, decoded once
        assert!(Arc::ptr_eq(&same.source, &base.source));
        assert_eq!(shown.of(Some(0)), Some(own));
        assert_eq!(shown.of(None), Some(base));
        assert_eq!(shown.of(Some(2)), None);
        assert_eq!(loaded.image_files().len(), 2);
    }
}
