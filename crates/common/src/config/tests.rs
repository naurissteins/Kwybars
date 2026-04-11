use super::{
    AppConfig, DaemonConfig, FrameMirrorMode, HorizontalAlignment, LineMode, MirrorOrientation,
    OverlayLayer, OverlayMonitorMode, OverlayPosition, VerticalAlignment, VisualizerBackend,
    VisualizerColorMode, VisualizerColorOverrides, VisualizerLayout, apply_color_overrides,
    parse_color_overrides, parse_config,
};

#[test]
fn parses_valid_config() {
    let raw = r#"
        [overlay]
        position = "top"
        layer = "top"
        anchor_margin = 20
        margin_left = 11
        margin_right = 13
        margin_top = 7
        margin_bottom = 9
        full_length = false
        width = 1200
        height = 140
        horizontal_alignment = "right"
        vertical_alignment = "bottom"
        monitor_mode = "list"
        monitors = ["DP-1", "HDMI-A-1"]

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
        color_rgba = "rgba(255, 255, 255, 0.5)"
        color2_rgba = "rgba(255, 0, 0, 1.0)"
        theme = "catppuccin-mocha"
        theme_opacity = 0.8
        pipewire_attack = 0.2
        pipewire_decay = 0.9
        pipewire_gain = 1.5
        pipewire_curve = 0.8
        pipewire_neighbor_mix = 0.3

        [daemon]
        enabled = true
        poll_interval_ms = 50
        activity_threshold = 0.045
        activate_delay_ms = 120
        deactivate_delay_ms = 1800
        stop_on_silence = false
        notify_on_error = true
        notify_cooldown_seconds = 30
        overlay_command = "cargo"
        overlay_args = ["run", "-p", "kwybars-overlay"]
        "#;

    let parsed = match parse_config(raw) {
        Ok(value) => value,
        Err(err) => panic!("valid config should parse, got error: {err}"),
    };
    assert_eq!(parsed.overlay.position, OverlayPosition::Top);
    assert_eq!(parsed.overlay.layer, OverlayLayer::Top);
    assert_eq!(parsed.overlay.anchor_margin, 20);
    assert_eq!(parsed.overlay.margin_left, 11);
    assert_eq!(parsed.overlay.margin_right, 13);
    assert_eq!(parsed.overlay.margin_top, 7);
    assert_eq!(parsed.overlay.margin_bottom, 9);
    assert!(!parsed.overlay.full_length);
    assert_eq!(parsed.overlay.width, 1200);
    assert_eq!(parsed.overlay.height, 140);
    assert_eq!(
        parsed.overlay.horizontal_alignment,
        HorizontalAlignment::Right
    );
    assert_eq!(parsed.overlay.vertical_alignment, VerticalAlignment::Bottom);
    assert_eq!(parsed.overlay.monitor_mode, OverlayMonitorMode::List);
    assert_eq!(parsed.overlay.monitors, vec!["DP-1", "HDMI-A-1"]);
    assert_eq!(parsed.visualizer.backend, VisualizerBackend::Dummy);
    assert_eq!(parsed.visualizer.layout, VisualizerLayout::Polygon);
    assert_eq!(parsed.visualizer.line_mode, LineMode::Split);
    assert_eq!(parsed.visualizer.line_split_gap, 220);
    assert_eq!(
        parsed.visualizer.mirror_orientation,
        MirrorOrientation::Vertical
    );
    assert_eq!(parsed.visualizer.mirror_gap, 24);
    assert_eq!(parsed.visualizer.wave_stroke_width, 6);
    assert!(!parsed.visualizer.wave_fill);
    assert!(parsed.visualizer.wave_glow);
    assert!((parsed.visualizer.wave_smoothing - 1.4).abs() < 1e-5);
    assert!((parsed.visualizer.wave_motion_smoothing - 0.3).abs() < 1e-5);
    assert!((parsed.visualizer.wave_amplitude - 1.25).abs() < 1e-5);
    assert_eq!(
        parsed.visualizer.frame_edges,
        vec![OverlayPosition::Top, OverlayPosition::Bottom]
    );
    assert_eq!(parsed.visualizer.frame_mirror_mode, FrameMirrorMode::Pairs);
    assert_eq!(parsed.visualizer.bars, 64);
    assert_eq!(parsed.visualizer.bar_width, 5);
    assert!((parsed.visualizer.bar_corner_radius - 3.5).abs() < 1e-5);
    assert!(parsed.visualizer.segmented_bars);
    assert_eq!(parsed.visualizer.segment_length, 9);
    assert_eq!(parsed.visualizer.segment_gap, 4);
    assert_eq!(parsed.visualizer.radial_inner_radius, 160);
    assert!((parsed.visualizer.radial_start_angle - (-180.0)).abs() < 1e-5);
    assert!((parsed.visualizer.radial_arc_degrees - 180.0).abs() < 1e-5);
    assert!((parsed.visualizer.radial_rotation_speed - 24.0).abs() < 1e-5);
    assert!((parsed.visualizer.center_offset_x - 80.0).abs() < 1e-5);
    assert!((parsed.visualizer.center_offset_y - (-40.0)).abs() < 1e-5);
    assert_eq!(parsed.visualizer.polygon_sides, 3);
    assert_eq!(parsed.visualizer.polygon_radius, 240);
    assert_eq!(parsed.visualizer.polygon_bar_length, 160);
    assert!((parsed.visualizer.polygon_rotation - (-90.0)).abs() < 1e-5);
    assert!((parsed.visualizer.polygon_rotation_speed - 12.0).abs() < 1e-5);
    assert_eq!(parsed.visualizer.gap, 2);
    assert_eq!(parsed.visualizer.framerate, 75);
    assert_eq!(parsed.visualizer.color_mode, VisualizerColorMode::Gradient);
    assert!((parsed.visualizer.color_rgba.r - 1.0).abs() < 1e-5);
    assert!((parsed.visualizer.color_rgba.g - 1.0).abs() < 1e-5);
    assert!((parsed.visualizer.color_rgba.b - 1.0).abs() < 1e-5);
    assert!((parsed.visualizer.color_rgba.a - 0.5).abs() < 1e-5);
    assert!((parsed.visualizer.color2_rgba.r - 1.0).abs() < 1e-5);
    assert!(parsed.visualizer.color2_rgba.g.abs() < 1e-5);
    assert!(parsed.visualizer.color2_rgba.b.abs() < 1e-5);
    assert!((parsed.visualizer.color2_rgba.a - 1.0).abs() < 1e-5);
    assert_eq!(parsed.visualizer.theme.as_deref(), Some("catppuccin-mocha"));
    assert!((parsed.visualizer.theme_opacity - 0.8).abs() < 1e-5);
    assert_eq!(parsed.visualizer.pipewire_attack, 0.2);
    assert_eq!(parsed.visualizer.pipewire_decay, 0.9);
    assert_eq!(parsed.visualizer.pipewire_gain, 1.5);
    assert_eq!(parsed.visualizer.pipewire_curve, 0.8);
    assert_eq!(parsed.visualizer.pipewire_neighbor_mix, 0.3);
    assert_eq!(
        parsed.daemon,
        DaemonConfig {
            enabled: true,
            poll_interval_ms: 50,
            activity_threshold: 0.045,
            activate_delay_ms: 120,
            deactivate_delay_ms: 1800,
            stop_on_silence: false,
            notify_on_error: true,
            notify_cooldown_seconds: 30,
            overlay_command: "cargo".to_owned(),
            overlay_args: vec![
                "run".to_owned(),
                "-p".to_owned(),
                "kwybars-overlay".to_owned()
            ],
        }
    );
}

#[test]
fn returns_default_for_empty_config() {
    let parsed = match parse_config("") {
        Ok(value) => value,
        Err(err) => panic!("empty config should parse, got error: {err}"),
    };
    assert_eq!(parsed, AppConfig::default());
}

#[test]
fn built_in_defaults_match_expected_no_config_setup() {
    let config = AppConfig::default();

    assert_eq!(config.overlay.monitor_mode, OverlayMonitorMode::Primary);
    assert_eq!(config.overlay.layer, OverlayLayer::Background);
    assert_eq!(config.overlay.position, OverlayPosition::Bottom);
    assert!(config.overlay.full_length);
    assert_eq!(config.overlay.height, 500);
    assert_eq!(config.overlay.anchor_margin, 20);
    assert_eq!(config.overlay.margin_left, 20);
    assert_eq!(config.overlay.margin_right, 20);

    assert_eq!(config.visualizer.backend, VisualizerBackend::Cava);
    assert_eq!(config.visualizer.layout, VisualizerLayout::Line);
    assert_eq!(config.visualizer.line_mode, LineMode::Continuous);
    assert_eq!(config.visualizer.line_split_gap, 200);
    assert_eq!(
        config.visualizer.mirror_orientation,
        MirrorOrientation::Horizontal
    );
    assert_eq!(config.visualizer.mirror_gap, 0);
    assert_eq!(config.visualizer.wave_stroke_width, 10);
    assert!(config.visualizer.wave_fill);
    assert!(!config.visualizer.wave_glow);
    assert!((config.visualizer.wave_smoothing - 1.0).abs() < 1e-5);
    assert!((config.visualizer.wave_motion_smoothing - 0.22).abs() < 1e-5);
    assert!((config.visualizer.wave_amplitude - 0.8).abs() < 1e-5);
    assert_eq!(
        config.visualizer.frame_edges,
        vec![OverlayPosition::Top, OverlayPosition::Bottom]
    );
    assert_eq!(config.visualizer.frame_mirror_mode, FrameMirrorMode::Pairs);
    assert!((config.visualizer.bar_corner_radius - 20.0).abs() < 1e-5);
    assert_eq!(config.visualizer.bars, 50);
    assert_eq!(config.visualizer.bar_width, 8);
    assert_eq!(config.visualizer.radial_inner_radius, 180);
    assert!((config.visualizer.radial_start_angle - (-90.0)).abs() < 1e-5);
    assert!((config.visualizer.radial_arc_degrees - 360.0).abs() < 1e-5);
    assert!(config.visualizer.radial_rotation_speed.abs() < 1e-5);
    assert!(config.visualizer.center_offset_x.abs() < 1e-5);
    assert!(config.visualizer.center_offset_y.abs() < 1e-5);
    assert_eq!(config.visualizer.polygon_sides, 3);
    assert_eq!(config.visualizer.polygon_radius, 220);
    assert_eq!(config.visualizer.polygon_bar_length, 0);
    assert!((config.visualizer.polygon_rotation - (-90.0)).abs() < 1e-5);
    assert!(config.visualizer.polygon_rotation_speed.abs() < 1e-5);
    assert_eq!(config.visualizer.gap, 20);
    assert_eq!(config.visualizer.framerate, 60);
    assert_eq!(config.visualizer.color_mode, VisualizerColorMode::Gradient);
    assert!((config.visualizer.color_rgba.r - (175.0 / 255.0)).abs() < 1e-5);
    assert!((config.visualizer.color_rgba.g - (198.0 / 255.0)).abs() < 1e-5);
    assert!((config.visualizer.color_rgba.b - 1.0).abs() < 1e-5);
    assert!((config.visualizer.color_rgba.a - 0.7).abs() < 1e-5);
    assert!((config.visualizer.color2_rgba.r - (191.0 / 255.0)).abs() < 1e-5);
    assert!((config.visualizer.color2_rgba.g - (198.0 / 255.0)).abs() < 1e-5);
    assert!((config.visualizer.color2_rgba.b - (220.0 / 255.0)).abs() < 1e-5);
    assert!((config.visualizer.color2_rgba.a - 0.7).abs() < 1e-5);

    assert!(config.daemon.enabled);
    assert_eq!(config.daemon.poll_interval_ms, 90);
    assert!((config.daemon.activity_threshold - 0.035).abs() < 1e-5);
    assert_eq!(config.daemon.activate_delay_ms, 180);
    assert_eq!(config.daemon.deactivate_delay_ms, 2200);
    assert!(config.daemon.stop_on_silence);
    assert!(config.daemon.notify_on_error);
    assert_eq!(config.daemon.notify_cooldown_seconds, 45);
    assert_eq!(config.daemon.overlay_command, "kwybars-overlay");
    assert!(config.daemon.overlay_args.is_empty());
}

#[test]
fn parses_colors_override_file() {
    let raw = r#"
        [colors]
        color_rgba = "rgba(10, 20, 30, 0.8)"
        color2_rgba = "rgba(100, 110, 120, 0.6)"
        "#;

    let parsed = match parse_color_overrides(raw) {
        Ok(value) => value,
        Err(err) => panic!("colors override should parse, got error: {err}"),
    };

    let Some(color1) = parsed.color_rgba else {
        panic!("missing color_rgba override");
    };
    assert!((color1.r - (10.0 / 255.0)).abs() < 1e-5);
    assert!((color1.g - (20.0 / 255.0)).abs() < 1e-5);
    assert!((color1.b - (30.0 / 255.0)).abs() < 1e-5);
    assert!((color1.a - 0.8).abs() < 1e-5);

    let Some(color2) = parsed.color2_rgba else {
        panic!("missing color2_rgba override");
    };
    assert!((color2.r - (100.0 / 255.0)).abs() < 1e-5);
    assert!((color2.g - (110.0 / 255.0)).abs() < 1e-5);
    assert!((color2.b - (120.0 / 255.0)).abs() < 1e-5);
    assert!((color2.a - 0.6).abs() < 1e-5);
}

#[test]
fn parses_legacy_frame_mirror_alias() {
    let raw = r#"
        [visualizer]
        layout = "frame"
        frame_mirror = true
        "#;

    let parsed = match parse_config(raw) {
        Ok(value) => value,
        Err(err) => panic!("legacy frame_mirror alias should parse, got error: {err}"),
    };

    assert_eq!(parsed.visualizer.layout, VisualizerLayout::Frame);
    assert_eq!(parsed.visualizer.frame_mirror_mode, FrameMirrorMode::All);
}

#[test]
fn display_tokens_round_trip_with_enum_parsers() {
    macro_rules! assert_round_trip {
            ($enum_type:ident [$($variant:ident),+ $(,)?]) => {
                $(
                    let value = $enum_type::$variant;
                    let token = value.to_string();
                    let reparsed = $enum_type::parse(&token)
                        .unwrap_or_else(|err| panic!(
                            "{} token `{}` should parse: {err}",
                            stringify!($enum_type),
                            token
                        ));
                    assert_eq!(reparsed, value);
                )+
            };
        }

    assert_round_trip!(OverlayPosition [Bottom, Top, Left, Right]);
    assert_round_trip!(OverlayLayer [Background, Bottom, Top]);
    assert_round_trip!(HorizontalAlignment [Left, Center, Right]);
    assert_round_trip!(VerticalAlignment [Top, Center, Bottom]);
    assert_round_trip!(OverlayMonitorMode [Primary, All, List]);
    assert_round_trip!(VisualizerBackend [Auto, Pipewire, Cava, Dummy]);
    assert_round_trip!(VisualizerColorMode [Solid, Gradient]);
    assert_round_trip!(VisualizerLayout [Line, Mirror, Wave, Frame, Radial, Polygon, Particle, Floating]);
    assert_round_trip!(LineMode [Continuous, Split]);
    assert_round_trip!(MirrorOrientation [Horizontal, Vertical]);
    assert_round_trip!(FrameMirrorMode [Off, All, Pairs]);
}

#[test]
fn parses_colors_override_with_inline_comment_and_separator() {
    let raw = r#"
        [visualizer]
        color_rgba = "rgba(202, 122, 99, 0.9)" # matugen
        color2_rgba = "rgba(150, 100, 255, 0.7)";
        "#;

    let parsed = match parse_color_overrides(raw) {
        Ok(value) => value,
        Err(err) => panic!("colors override should parse, got error: {err}"),
    };

    let Some(color1) = parsed.color_rgba else {
        panic!("missing color_rgba override");
    };
    assert!((color1.r - (202.0 / 255.0)).abs() < 1e-5);
    assert!((color1.g - (122.0 / 255.0)).abs() < 1e-5);
    assert!((color1.b - (99.0 / 255.0)).abs() < 1e-5);
    assert!((color1.a - 0.9).abs() < 1e-5);

    let Some(color2) = parsed.color2_rgba else {
        panic!("missing color2_rgba override");
    };
    assert!((color2.r - (150.0 / 255.0)).abs() < 1e-5);
    assert!((color2.g - (100.0 / 255.0)).abs() < 1e-5);
    assert!((color2.b - 1.0).abs() < 1e-5);
    assert!((color2.a - 0.7).abs() < 1e-5);
}

#[test]
fn applies_color_overrides_with_precedence() {
    let mut config = AppConfig::default();
    let original_color2 = config.visualizer.color2_rgba;

    let overrides = VisualizerColorOverrides {
        color_rgba: Some(super::RgbaColor {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 0.75,
        }),
        color2_rgba: None,
    };
    apply_color_overrides(&mut config, overrides);

    assert!((config.visualizer.color_rgba.r - 1.0).abs() < 1e-5);
    assert!(config.visualizer.color_rgba.g.abs() < 1e-5);
    assert!(config.visualizer.color_rgba.b.abs() < 1e-5);
    assert!((config.visualizer.color_rgba.a - 0.75).abs() < 1e-5);
    assert_eq!(config.visualizer.color2_rgba, original_color2);
}

#[test]
fn parses_root_theme_keys() {
    let raw = r#"
        theme = "catppuccin-mocha"
        theme_opacity = 0.7

        [overlay]
        position = "bottom"
        "#;

    let parsed = match parse_config(raw) {
        Ok(value) => value,
        Err(err) => panic!("root theme keys should parse, got error: {err}"),
    };

    assert_eq!(parsed.visualizer.theme.as_deref(), Some("catppuccin-mocha"));
    assert!((parsed.visualizer.theme_opacity - 0.7).abs() < 1e-5);
}
