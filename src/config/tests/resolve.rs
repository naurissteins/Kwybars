use super::parse_ok;
use crate::config::{ColorOverrides, Edge, Layout, Rgba, Theme};

const NORD: &str = "red = \"#bf616a\"\ngreen = \"#a3be8c\"\nyellow = \"#ebcb8b\"\nblue = \"#81a1c1\"\nmagenta = \"#b48ead\"\ncyan = \"#88c0d0\"\n";

fn nord() -> Theme {
    Theme::parse(NORD, "nord").unwrap_or_else(|err| panic!("{err}"))
}

const OUTPUTS: &str = r#"
[overlay]
position = "bottom"
height = 300

[visualizer]
layout = "line"
gap = 10
theme_opacity = 0.5

[[overlay.outputs]]
monitor = "DP-1"
position = "top"
fade_in_ms = 50

[overlay.outputs.visualizer]
layout = "wave"

[[overlay.outputs]]
monitor = "DP-2"

[overlay.outputs.visualizer]
color_rgba = "rgba(255, 0, 0, 1)"
"#;

#[test]
fn base_surface_uses_overlay_and_theme() {
    let config = parse_ok(OUTPUTS).config;
    let surface = config.surface(None, Some(&nord()));
    assert_eq!(
        (surface.overlay.position, surface.overlay.height),
        (Edge::Bottom, 300)
    );
    assert_eq!(surface.visualizer.layout, Layout::Line);
    let colors = surface
        .theme_colors
        .unwrap_or_else(|| panic!("theme should apply"));
    assert!(colors.iter().all(|color| (color.a - 0.5).abs() < 1e-6));
}

#[test]
fn output_overrides_merge_over_the_base() {
    let config = parse_ok(OUTPUTS).config;
    let surface = config.surface(config.overlay.outputs.first(), Some(&nord()));
    assert_eq!(surface.overlay.position, Edge::Top);
    assert_eq!(
        (surface.overlay.height, surface.overlay.fade_in_ms),
        (300, 50)
    );
    assert_eq!(
        (surface.visualizer.layout, surface.visualizer.gap),
        (Layout::Wave, 10)
    );
    assert!(surface.theme_colors.is_some());
}

#[test]
fn output_colors_disable_the_theme() {
    let config = parse_ok(OUTPUTS).config;
    let surface = config.surface(config.overlay.outputs.get(1), Some(&nord()));
    assert!(surface.theme_colors.is_none());
    assert_eq!(surface.visualizer.color_rgba, Rgba::new(1.0, 0.0, 0.0, 1.0));
}

#[test]
fn output_colors_win_over_colors_toml() {
    let mut config = parse_ok(OUTPUTS).config;
    let blue = Rgba::new(0.0, 0.0, 1.0, 1.0);
    ColorOverrides {
        color_rgba: Some(blue),
        color2_rgba: Some(blue),
    }
    .apply_to(&mut config.visualizer);

    let base = config.surface(None, None);
    assert_eq!(
        (base.visualizer.color_rgba, base.visualizer.color2_rgba),
        (blue, blue)
    );
    let output = config.surface(config.overlay.outputs.get(1), None);
    assert_eq!(output.visualizer.color_rgba, Rgba::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(output.visualizer.color2_rgba, blue);
}
