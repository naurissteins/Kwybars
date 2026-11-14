use super::TempDir;
use crate::config::{self, Config};
use crate::xdg::fake_env;

#[test]
fn background_defaults_and_per_output_overrides_are_independent()
-> Result<(), Box<dyn std::error::Error>> {
    let parsed = config::parse(
        r#"
[overlay]
layer = "bottom"
[background]
enabled = true
path = "wall.png"
sensitivity = 9.0
[output.DP-1.background]
path = "other.png"
zoom = 0.08
[output.DP-2.background]
enabled = false
"#,
    )?;
    assert!(parsed.warnings.is_empty());
    let config = parsed.config;
    let one = config.background(config.overlay.outputs.first());
    assert_eq!(one.path.as_deref(), Some("other.png"));
    assert_eq!((one.sensitivity, one.zoom), (9.0, 0.08));
    assert!(!config.background(config.overlay.outputs.get(1)).enabled);
    assert!(!Config::default().has_background());
    Ok(())
}

#[test]
fn invalid_background_values_are_normalized_and_named() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = config::parse(
        r#"
[background]
zoom = nan
shake = 1.0
low_cutoff_hz = 1000
high_cutoff_hz = 40
attack_ms = 0
[output.DP-1.background]
low_cutoff_hz = 90
"#,
    )?;
    assert_eq!(parsed.config.background.zoom, 0.04);
    assert_eq!(parsed.config.background.shake, 0.02);
    assert_eq!(parsed.config.background.high_cutoff_hz, 2000.0);
    assert_eq!(parsed.config.background.attack_ms, 1);
    assert!(
        parsed
            .warnings
            .iter()
            .any(|warning| warning.contains("output.DP-1.background.low_cutoff_hz"))
    );
    assert!(
        parsed
            .config
            .overlay
            .outputs
            .first()
            .is_some_and(|output| output.background.low_cutoff_hz.is_none())
    );
    assert!(config::parse("[background]\nzoom = \"wrong\"").is_err());
    Ok(())
}

#[test]
fn background_images_share_decoding_and_are_watched_and_reused()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new("background-load");
    let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 10, 20, 255]));
    image.save(dir.path().join("wall.png"))?;
    let path = dir.write("config.toml", "[background]\nenabled = true\npath = \"wall.png\"\n[output.DP-1.background]\nzoom = 0.08\n");
    let first = config::load(&path, &fake_env(&[]))?;
    assert!(first.problems.is_empty());
    let base = first
        .background
        .as_ref()
        .and_then(|image| image.source.as_ref().ok())
        .ok_or("no base image")?;
    let own = first
        .output_backgrounds
        .first()
        .and_then(Option::as_ref)
        .and_then(|image| image.source.as_ref().ok())
        .ok_or("no output image")?;
    assert!(std::sync::Arc::ptr_eq(base, own));
    assert_eq!(first.image_files(), vec![dir.path().join("wall.png")]);
    let known: Vec<_> = first.background_images().cloned().collect();
    let again = config::load_reusing(&path, &fake_env(&[]), &[], &known)?;
    let again = again
        .background
        .as_ref()
        .and_then(|image| image.source.as_ref().ok())
        .ok_or("no reloaded image")?;
    assert!(std::sync::Arc::ptr_eq(base, again));
    Ok(())
}

#[test]
fn sweetbg_source_is_opt_in_inherited_and_does_not_require_a_path()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::config::BackgroundSource;
    let dir = TempDir::new("sweetbg-config");
    let path = dir.write("config.toml", "[background]\nenabled = true\nsource = \"sweetbg\"\n[output.DP-2.background]\nsource = \"file\"\nenabled = false\n");
    let loaded = config::load(&path, &fake_env(&[]))?;
    assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
    assert!(loaded.config.uses_sweetbg());
    assert_eq!(loaded.config.background.source, BackgroundSource::Sweetbg);
    assert_eq!(
        loaded
            .config
            .background(loaded.config.overlay.outputs.first())
            .source,
        BackgroundSource::File
    );
    assert_eq!(Config::default().background.source, BackgroundSource::File);
    assert!(config::parse("[background]\nsource = \"unknown\"").is_err());
    Ok(())
}
