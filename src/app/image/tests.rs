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

#[test]
fn background_failure_keeps_the_named_outputs_previous_image() {
    use super::backgrounds;
    let dir = TempDir::new("app-background");
    save(&dir, "a.png", 1);
    save(&dir, "b.png", 2);
    let first = load(
        &dir,
        "[background]\nenabled = true\npath = \"a.png\"\n[output.DP-2.background]\npath = \"b.png\"\n",
    );
    let shown = backgrounds(&first, &Overlays::default(), &first.config);
    dir.write("b.png", "broken image");
    let next = load(
        &dir,
        "[background]\nenabled = true\npath = \"a.png\"\n[output.DP-1.background]\nzoom = 0.06\n[output.DP-2.background]\npath = \"b.png\"\n",
    );
    let updated = backgrounds(&next, &shown, &first.config);
    assert!(
        matches!((shown.of(Some(0)), updated.of(Some(1))), (Some(old), Some(new)) if Arc::ptr_eq(&old.source, &new.source))
    );
}
