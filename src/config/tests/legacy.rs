//! the new parser must agree with the legacy one on every shared key

use std::path::{Path, PathBuf};

use kwybars_common::config as legacy;

use crate::config::{self, Config, OutputConfig, Rgba};
use crate::xdg::fake_env;

macro_rules! same {
    ($file:expr, $new:expr, $old:expr, [$($field:ident),+ $(,)?]) => {
        $(
            assert_eq!(
                format!("{:?}", $new.$field),
                format!("{:?}", $old.$field),
                "{}: {} differs",
                $file,
                stringify!($field)
            );
        )+
    };
}

fn rgba(color: Rgba) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

fn legacy_rgba(color: legacy::RgbaColor) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

fn assert_matches(file: &str, new: &Config, old: &legacy::AppConfig) {
    let (overlay, old_overlay) = (&new.overlay, &old.overlay);
    same!(
        file,
        overlay,
        old_overlay,
        [
            position,
            layer,
            anchor_margin,
            margin_left,
            margin_right,
            margin_top,
            margin_bottom,
            fade_in_ms,
            fade_out_ms,
            full_length,
            width,
            height,
            horizontal_alignment,
            vertical_alignment,
            monitor_mode,
            monitors,
        ]
    );
    assert_eq!(
        overlay.outputs.len(),
        old_overlay.outputs.len(),
        "{file}: outputs"
    );
    for (output, old_output) in overlay.outputs.iter().zip(&old_overlay.outputs) {
        assert_output_matches(file, output, old_output);
    }

    let (v, old_v) = (&new.visualizer, &old.visualizer);
    same!(
        file,
        v,
        old_v,
        [
            layout,
            line_mode,
            line_split_gap,
            mirror_orientation,
            mirror_gap,
            wave_stroke_width,
            wave_fill,
            wave_glow,
            wave_smoothing,
            wave_motion_smoothing,
            wave_amplitude,
            frame_edges,
            frame_mirror_mode,
            bars,
            bar_width,
            bar_corner_radius,
            segmented_bars,
            segment_length,
            segment_gap,
            radial_inner_radius,
            radial_start_angle,
            radial_arc_degrees,
            radial_rotation_speed,
            center_offset_x,
            center_offset_y,
            polygon_sides,
            polygon_radius,
            polygon_bar_length,
            polygon_rotation,
            polygon_rotation_speed,
            gap,
            framerate,
            color_mode,
            gradient_direction,
            theme,
            theme_opacity,
        ]
    );
    assert_eq!(
        rgba(v.color_rgba),
        legacy_rgba(old_v.color_rgba),
        "{file}: color_rgba"
    );
    assert_eq!(
        rgba(v.color2_rgba),
        legacy_rgba(old_v.color2_rgba),
        "{file}: color2_rgba"
    );

    let (activity, daemon) = (&new.activity, &old.daemon);
    assert_eq!(
        format!(
            "{:?}",
            (
                activity.threshold,
                activity.activate_delay_ms,
                activity.deactivate_delay_ms
            )
        ),
        format!(
            "{:?}",
            (
                daemon.activity_threshold,
                daemon.activate_delay_ms,
                daemon.deactivate_delay_ms
            )
        ),
        "{file}: activity"
    );

    let (image, old_image) = (&new.image_overlay, &old.image_overlay);
    same!(
        file,
        image,
        old_image,
        [
            enabled, path, opacity, fit, width, height, offset_x, offset_y
        ]
    );
}

fn assert_output_matches(file: &str, new: &OutputConfig, old: &legacy::OverlayOutputConfig) {
    same!(
        file,
        new,
        old,
        [
            monitor,
            enabled,
            position,
            layer,
            anchor_margin,
            margin_left,
            margin_right,
            margin_top,
            margin_bottom,
            fade_in_ms,
            fade_out_ms,
            full_length,
            width,
            height,
            horizontal_alignment,
            vertical_alignment,
        ]
    );
    let (v, old_v) = (&new.visualizer, &old.visualizer);
    same!(
        file,
        v,
        old_v,
        [
            layout,
            line_mode,
            line_split_gap,
            mirror_orientation,
            mirror_gap,
            wave_stroke_width,
            wave_fill,
            wave_glow,
            wave_smoothing,
            wave_motion_smoothing,
            wave_amplitude,
            frame_edges,
            frame_mirror_mode,
            bar_width,
            bar_corner_radius,
            segmented_bars,
            segment_length,
            segment_gap,
            radial_inner_radius,
            radial_start_angle,
            radial_arc_degrees,
            radial_rotation_speed,
            center_offset_x,
            center_offset_y,
            polygon_sides,
            polygon_radius,
            polygon_bar_length,
            polygon_rotation,
            polygon_rotation_speed,
            gap,
            color_mode,
            gradient_direction,
        ]
    );
    assert_eq!(
        v.color_rgba.map(rgba),
        old_v.color_rgba.map(legacy_rgba),
        "{file}: output color_rgba"
    );
    assert_eq!(
        v.color2_rgba.map(rgba),
        old_v.color2_rgba.map(legacy_rgba),
        "{file}: output color2_rgba"
    );
}

fn compare_file(path: &Path) {
    let name = path.display().to_string();
    let new = match config::load(path, &fake_env(&[])) {
        Ok(loaded) => loaded.config,
        Err(err) => panic!("{name}: new parser failed: {err}"),
    };
    let old = match legacy::load_or_default(path) {
        Ok(config) => config,
        Err(err) => panic!("{name}: legacy parser failed: {err}"),
    };
    assert_matches(&name, &new, &old);
}

fn toml_files(dir: &Path) -> Vec<PathBuf> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => panic!("cannot read {}: {err}", dir.display()),
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    files
}

#[test]
fn example_configs_match_legacy_parser() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/examples");
    let files = toml_files(&dir);
    assert!(
        files.len() >= 9,
        "expected the bundled examples, found {files:?}"
    );
    for file in files {
        compare_file(&file);
    }
}

/// `KWYBARS_COMPARE_DIR=~/.config/kwybars cargo test -- --ignored matches_legacy`
#[test]
#[ignore = "needs KWYBARS_COMPARE_DIR"]
fn configs_in_dir_match_legacy_parser() {
    let Some(dir) = std::env::var_os("KWYBARS_COMPARE_DIR") else {
        panic!("set KWYBARS_COMPARE_DIR to a directory of config files");
    };
    for file in toml_files(Path::new(&dir)) {
        compare_file(&file);
    }
}
