use super::{assert_close, parse_err, parse_ok};
use crate::config::{
    ColorMode, Config, Edge, FrameMirrorMode, GradientDirection, HorizontalAlignment, ImageFit,
    Layer, Layout, LineMode, MirrorOrientation, Rgba, ShowOn, VerticalAlignment,
};

const FULL: &str = r#"
[overlay]
position = "top"
layer = "top"
anchor_margin = 20
margin_left = 11
margin_right = 13
margin_top = 7
margin_bottom = 9
fade_in_ms = 150
fade_out_ms = 420
full_length = false
width = 1200
height = 140
horizontal_alignment = "right"
vertical_alignment = "bottom"
monitor_mode = "list"
monitors = ["DP-1", " HDMI-A-1 ", ""]

[visualizer]
backend = "dummy"
layout = "polygon"
line_mode = "split"
line_split_gap = 220
mirror_orientation = "vertical"
mirror_gap = 24
wave_stroke_width = 6
wave_fill = false
wave_glow = true
wave_smoothing = 1.4
wave_motion_smoothing = 0.3
wave_amplitude = 1.25
frame_edges = ["top", "bottom"]
frame_mirror_mode = "pairs"
bars = 64
bar_width = 5
bar_corner_radius = 3.5
segmented_bars = true
segment_length = 9
segment_gap = 4
radial_inner_radius = 160
radial_start_angle = -180
radial_arc_degrees = 180
radial_rotation_speed = 24
center_offset_x = 80
center_offset_y = -40
polygon_sides = 3
polygon_radius = 240
polygon_bar_length = 160
polygon_rotation = -90
polygon_rotation_speed = 12
gap = 2
framerate = 75
color_mode = "gradient"
gradient_direction = "horizontal"
color_rgba = "rgba(255, 255, 255, 0.5)"
color2_rgba = "rgba(255, 0, 0, 1.0)"
theme = "catppuccin-mocha"
theme_opacity = 0.8
pipewire_attack = 0.2

[image_overlay]
enabled = true
path = "~/Pictures/kwybars/mountain.png"
opacity = 0.82
fit = "cover"
width = 1200
height = 260
offset_x = 12
offset_y = -8

[daemon]
enabled = true
activity_threshold = 0.045

[[overlay.outputs]]
monitor = "DP-1"
position = "bottom"
height = 180
margin_left = 40
fade_out_ms = 500

[overlay.outputs.visualizer]
layout = "wave"
wave_glow = true
wave_stroke_width = 5
color_mode = "solid"
color_rgba = "rgba(10, 20, 30, 0.8)"

[[overlay.outputs]]
monitor = "HDMI-A-1"
enabled = false
"#;

#[test]
fn parses_every_section() {
    let parsed = parse_ok(FULL);
    let overlay = &parsed.config.overlay;
    assert_eq!(
        (overlay.position, overlay.layer, overlay.anchor_margin),
        (Edge::Top, Layer::Top, 20)
    );
    assert_eq!(
        (
            overlay.margin_left,
            overlay.margin_right,
            overlay.margin_top,
            overlay.margin_bottom
        ),
        (11, 13, 7, 9)
    );
    assert_eq!((overlay.fade_in_ms, overlay.fade_out_ms), (150, 420));
    assert_eq!(
        (overlay.full_length, overlay.width, overlay.height),
        (false, 1200, 140)
    );
    assert_eq!(overlay.horizontal_alignment, HorizontalAlignment::Right);
    assert_eq!(overlay.vertical_alignment, VerticalAlignment::Bottom);
    // the old per-output entries decide, as they always did
    assert_eq!(overlay.show_on, ShowOn::Sections);

    let [first, second] = overlay.outputs.as_slice() else {
        panic!("expected two outputs, got {:?}", overlay.outputs);
    };
    assert_eq!(first.monitor, "DP-1");
    assert!(first.enabled);
    assert_eq!(
        (
            first.position,
            first.height,
            first.margin_left,
            first.fade_out_ms
        ),
        (Some(Edge::Bottom), Some(180), Some(40), Some(500))
    );
    assert_eq!(first.visualizer.layout, Some(Layout::Wave));
    assert_eq!(first.visualizer.wave_glow, Some(true));
    assert_eq!(first.visualizer.wave_stroke_width, Some(5));
    assert_eq!(first.visualizer.color_mode, Some(ColorMode::Solid));
    assert_eq!(
        first.visualizer.color_rgba,
        Some(Rgba::new(10.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0, 0.8))
    );
    assert_eq!(second.monitor, "HDMI-A-1");
    assert!(!second.enabled);

    let v = &parsed.config.visualizer;
    assert_eq!(v.layout, Layout::Polygon);
    assert_eq!((v.line_mode, v.line_split_gap), (LineMode::Split, 220));
    assert_eq!(
        (v.mirror_orientation, v.mirror_gap),
        (MirrorOrientation::Vertical, 24)
    );
    assert_eq!(
        (v.wave_stroke_width, v.wave_fill, v.wave_glow),
        (6, false, true)
    );
    assert_close(v.wave_smoothing, 1.4);
    assert_close(v.wave_motion_smoothing, 0.3);
    assert_close(v.wave_amplitude, 1.25);
    assert_eq!(v.frame_edges, vec![Edge::Top, Edge::Bottom]);
    assert_eq!(v.frame_mirror_mode, FrameMirrorMode::Pairs);
    assert_eq!((v.bars, v.bar_width), (64, 5));
    assert_close(v.bar_corner_radius, 3.5);
    assert_eq!(
        (v.segmented_bars, v.segment_length, v.segment_gap),
        (true, 9, 4)
    );
    assert_eq!(v.radial_inner_radius, 160);
    assert_close(v.radial_start_angle, -180.0);
    assert_close(v.radial_arc_degrees, 180.0);
    assert_close(v.radial_rotation_speed, 24.0);
    assert_close(v.center_offset_x, 80.0);
    assert_close(v.center_offset_y, -40.0);
    assert_eq!(
        (v.polygon_sides, v.polygon_radius, v.polygon_bar_length),
        (3, 240, 160)
    );
    assert_close(v.polygon_rotation, -90.0);
    assert_close(v.polygon_rotation_speed, 12.0);
    assert_eq!((v.gap, v.framerate), (2, 75));
    assert_eq!(v.color_mode, ColorMode::Gradient);
    assert_eq!(v.gradient_direction, GradientDirection::Horizontal);
    assert_eq!(v.color_rgba, Rgba::new(1.0, 1.0, 1.0, 0.5));
    assert_eq!(v.color2_rgba, Rgba::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(v.theme.as_deref(), Some("catppuccin-mocha"));
    assert_close(v.theme_opacity, 0.8);

    let image = &parsed.config.image_overlay;
    assert!(image.enabled);
    assert_eq!(
        image.path.as_deref(),
        Some("~/Pictures/kwybars/mountain.png")
    );
    assert_close(image.opacity, 0.82);
    assert_eq!(
        (image.fit, image.width, image.height),
        (ImageFit::Cover, 1200, 260)
    );
    assert_close(image.offset_x, 12.0);
    assert_close(image.offset_y, -8.0);
}

#[test]
fn unknown_keys_warn_with_their_path() {
    let parsed = parse_ok(FULL);
    assert_eq!(
        parsed.warnings,
        vec![
            "daemon.enabled: removed (kwybars runs as a single process without a daemon), ignored",
            "visualizer.backend, visualizer.pipewire_attack: removed (audio is captured from PipeWire directly), ignored",
            "[[overlay.outputs]] is deprecated, write each entry as its own section, for example [output.DP-1]",
            "overlay.monitor_mode, overlay.monitors: deprecated and not used here, `show_on` or the [output.NAME] sections choose the monitors",
            "[daemon] is deprecated, move these keys to [activity]: daemon.activity_threshold -> activity.threshold",
        ]
    );

    let parsed = parse_ok(
        "[[overlay.outputs]]\nmonitor = \"DP-1\"\nfoo = 1\n[overlay.outputs.visualizer]\nbar = 2\n",
    );
    assert_eq!(
        parsed.warnings,
        vec![
            "overlay.outputs[0].foo: unknown key, ignored",
            "overlay.outputs[0].visualizer.bar: unknown key, ignored",
            "[[overlay.outputs]] is deprecated, write each entry as its own section, for example [output.DP-1]",
        ]
    );
}

#[test]
fn empty_config_is_the_default() {
    let parsed = parse_ok("");
    assert_eq!(parsed.config, Config::default());
    assert!(parsed.warnings.is_empty());
}

#[test]
fn defaults_match_the_documented_values() {
    let config = Config::default();
    let overlay = &config.overlay;
    assert_eq!(overlay.show_on, ShowOn::Primary);
    assert_eq!(
        (overlay.layer, overlay.position),
        (Layer::Background, Edge::Bottom)
    );
    assert!(overlay.full_length);
    assert_eq!((overlay.height, overlay.anchor_margin), (500, 20));
    assert_eq!((overlay.margin_left, overlay.margin_right), (20, 20));
    assert_eq!((overlay.fade_in_ms, overlay.fade_out_ms), (180, 350));

    let v = &config.visualizer;
    assert_eq!(
        (v.layout, v.bars, v.bar_width, v.gap, v.framerate),
        (Layout::Line, 50, 8, 20, 60)
    );
    assert_close(v.bar_corner_radius, 20.0);
    assert_eq!(v.frame_mirror_mode, FrameMirrorMode::Pairs);
    assert_eq!(v.color_mode, ColorMode::Gradient);
    assert_eq!(
        v.color_rgba,
        Rgba::new(175.0 / 255.0, 198.0 / 255.0, 1.0, 0.7)
    );
    assert!(v.theme.is_none());
    assert!(!config.image_overlay.enabled);
}

#[test]
fn output_without_monitor_is_an_error() {
    let err = parse_err("[[overlay.outputs]]\nposition = \"bottom\"\n");
    assert_eq!(err, "overlay.outputs[0]: missing `monitor`");
}

#[test]
fn global_keys_in_output_visualizer_are_dropped() {
    let parsed = parse_ok("[output.DP-1.visualizer]\nbars = 80\ngap = 4\n");
    let output = &parsed.config.overlay.outputs[0].visualizer;
    assert_eq!((output.bars, output.gap), (None, Some(4)));
    assert_eq!(
        parsed.warnings,
        vec!["output.DP-1.visualizer.bars: cannot be set per output, ignored"]
    );
}

#[test]
fn legacy_frame_mirror_alias() {
    let on = parse_ok("[visualizer]\nlayout = \"frame\"\nframe_mirror = true\n");
    assert_eq!(on.config.visualizer.frame_mirror_mode, FrameMirrorMode::All);
    let off = parse_ok("[visualizer]\nframe_mirror = false\n");
    assert_eq!(
        off.config.visualizer.frame_mirror_mode,
        FrameMirrorMode::Off
    );
}

#[test]
fn root_theme_keys_are_a_shorthand() {
    let parsed = parse_ok("theme = \"nord\"\ntheme_opacity = 0.7\n[overlay]\n");
    assert_eq!(parsed.config.visualizer.theme.as_deref(), Some("nord"));
    assert_close(parsed.config.visualizer.theme_opacity, 0.7);

    let parsed = parse_ok("theme = \"nord\"\n[visualizer]\ntheme = \"dracula\"\n");
    assert_eq!(parsed.config.visualizer.theme.as_deref(), Some("dracula"));
}

#[test]
fn integers_are_accepted_for_float_keys() {
    let parsed = parse_ok("[visualizer]\nradial_start_angle = 0\nwave_amplitude = 2\n");
    assert_close(parsed.config.visualizer.radial_start_angle, 0.0);
    assert_close(parsed.config.visualizer.wave_amplitude, 2.0);
}

#[test]
fn out_of_range_values_are_fixed_with_a_warning() {
    let parsed = parse_ok(
        "[visualizer]\npolygon_sides = 2\ntheme_opacity = 1.5\nbars = 0\nwave_amplitude = -1\nradial_start_angle = nan\n[image_overlay]\nopacity = 3\n",
    );
    let v = &parsed.config.visualizer;
    assert_eq!((v.polygon_sides, v.bars), (3, 1));
    assert_close(v.theme_opacity, 1.0);
    assert_close(v.wave_amplitude, 0.0);
    assert_close(v.radial_start_angle, -90.0);
    assert_close(parsed.config.image_overlay.opacity, 1.0);
    assert_eq!(parsed.warnings.len(), 6, "{:?}", parsed.warnings);
}

#[test]
fn frame_edges_are_deduplicated_and_never_empty() {
    let parsed = parse_ok("[visualizer]\nframe_edges = [\"left\", \"right\", \"left\"]\n");
    assert_eq!(
        parsed.config.visualizer.frame_edges,
        vec![Edge::Left, Edge::Right]
    );
    let parsed = parse_ok("[visualizer]\nframe_edges = []\n");
    assert_eq!(
        parsed.config.visualizer.frame_edges,
        vec![Edge::Top, Edge::Bottom]
    );
}

#[test]
fn empty_strings_mean_unset() {
    let parsed = parse_ok("[visualizer]\ntheme = \"  \"\n[image_overlay]\npath = \"\"\n");
    assert!(parsed.config.visualizer.theme.is_none());
    assert!(parsed.config.image_overlay.path.is_none());
}

#[test]
fn type_errors_name_the_key_and_line() {
    let err = parse_err("[overlay]\nposition = \"bottom\"\nheight = \"tall\"\n");
    assert!(err.contains("line 3"), "{err}");
    assert!(err.contains("height"), "{err}");
}

#[test]
fn unknown_enum_values_are_errors() {
    let err = parse_err("[visualizer]\nlayout = \"spiral\"\n");
    assert!(err.contains("spiral") && err.contains("line 2"), "{err}");
}

#[test]
fn bad_colors_are_errors() {
    let err = parse_err("[visualizer]\ncolor_rgba = \"rgba(1, 2)\"\n");
    assert!(
        err.contains("invalid color") && err.contains("line 2"),
        "{err}"
    );
}

#[test]
fn duplicate_keys_are_errors() {
    let err = parse_err("[visualizer]\nbars = 20\nbars = 30\n");
    assert!(err.contains("duplicate") && err.contains("line 3"), "{err}");
}
