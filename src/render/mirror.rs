//! `mirror`: bars growing both ways from a center line, as two strips back
//! to back, following the legacy overlay

use super::fill::Axis;
use super::line::{Region, Strip};
use crate::config::{Edge, GradientDirection, MirrorOrientation, SurfaceConfig};

/// shortest half bar, in logical pixels, as in the legacy overlay
const MIN_EXTENT: f32 = 1.0;

/// the two halves and the gradient axis for a buffer of size
pub fn strips(config: &SurfaceConfig, size: (u32, u32), scale: f32) -> ([Strip; 2], Axis) {
    let visualizer = &config.visualizer;
    let horizontal = visualizer.mirror_orientation == MirrorOrientation::Horizontal;
    let active = active_region(config, size, scale, horizontal);
    let depth = if horizontal {
        active.height
    } else {
        active.width
    };
    let gap = (visualizer.mirror_gap as f32 * scale).clamp(0.0, (depth - 2.0 * scale).max(0.0));
    let half = ((depth - gap) * 0.5).max(scale);
    let strip = |edge, region| Strip {
        edge,
        region,
        min_extent: MIN_EXTENT,
        mode: visualizer.line_mode,
    };
    let halves = if horizontal {
        let center = active.y + active.height * 0.5;
        [
            strip(
                Edge::Bottom,
                Region {
                    y: center - gap * 0.5 - half,
                    height: half,
                    ..active
                },
            ),
            strip(
                Edge::Top,
                Region {
                    y: center + gap * 0.5,
                    height: half,
                    ..active
                },
            ),
        ]
    } else {
        let center = active.x + active.width * 0.5;
        [
            strip(
                Edge::Right,
                Region {
                    x: center - gap * 0.5 - half,
                    width: half,
                    ..active
                },
            ),
            strip(
                Edge::Left,
                Region {
                    x: center + gap * 0.5,
                    width: half,
                    ..active
                },
            ),
        ]
    };
    (
        halves,
        axis(active, horizontal, visualizer.gradient_direction),
    )
}

fn active_region(
    config: &SurfaceConfig,
    (width, height): (u32, u32),
    scale: f32,
    horizontal: bool,
) -> Region {
    let overlay = &config.overlay;
    let (width, height) = (width as f32, height as f32);
    let offset_x = config.visualizer.center_offset_x * scale;
    let offset_y = config.visualizer.center_offset_y * scale;
    let configured_width = (overlay.width as f32 * scale).min(width.max(1.0));
    let configured_height = (overlay.height as f32 * scale).min(height.max(1.0));
    let full = overlay.full_length;
    let (active_width, active_height) = match (horizontal, full) {
        (true, true) => (width.max(1.0), configured_height),
        (false, true) => (configured_width, height.max(1.0)),
        (_, false) => (configured_width, configured_height),
    };
    let x = if horizontal && full {
        offset_x
    } else {
        (width - active_width) * 0.5 + offset_x
    };
    let y = if !horizontal && full {
        offset_y
    } else {
        (height - active_height) * 0.5 + offset_y
    };
    Region {
        x,
        y,
        width: active_width,
        height: active_height,
    }
}

/// legacy's mirror gradient: across the active band from its start, along the
/// row of bars for a horizontal gradient and across it for a vertical one
fn axis(active: Region, horizontal: bool, direction: GradientDirection) -> Axis {
    let columns = horizontal == (direction == GradientDirection::Horizontal);
    let (start, length) = if columns {
        (active.x, active.width)
    } else {
        (active.y, active.height)
    };
    Axis {
        columns,
        start,
        length,
        reversed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::strips;
    use crate::config::{Config, Edge, GradientDirection, MirrorOrientation, SurfaceConfig};

    fn config(
        orientation: MirrorOrientation,
        gap: u32,
        edit: impl FnOnce(&mut Config),
    ) -> SurfaceConfig {
        let mut config = Config::default();
        config.visualizer.mirror_orientation = orientation;
        config.visualizer.mirror_gap = gap;
        config.overlay.full_length = false;
        config.overlay.width = 200;
        config.overlay.height = 100;
        edit(&mut config);
        config.surface(None, None)
    }

    #[test]
    fn horizontal_halves_share_the_height_symmetrically() {
        let config = config(MirrorOrientation::Horizontal, 0, |_| {});
        let ([top, bottom], _) = strips(&config, (400, 300), 1.0);
        // a 200x100 band centered in 400x300
        assert_eq!((top.edge, bottom.edge), (Edge::Bottom, Edge::Top));
        assert_eq!((top.region.x, top.region.width), (100.0, 200.0));
        assert_eq!((top.region.y, top.region.height), (100.0, 50.0));
        assert_eq!((bottom.region.y, bottom.region.height), (150.0, 50.0));
    }

    #[test]
    fn the_gap_offsets_both_halves_from_the_center() {
        let config = config(MirrorOrientation::Horizontal, 20, |_| {});
        let ([top, bottom], _) = strips(&config, (200, 100), 1.0);
        assert_eq!((top.region.y, top.region.height), (0.0, 40.0));
        assert_eq!((bottom.region.y, bottom.region.height), (60.0, 40.0));
        let vertical = self::config(MirrorOrientation::Vertical, 20, |c| {
            c.overlay.width = 100;
            c.overlay.height = 200;
        });
        let ([left, right], _) = strips(&vertical, (100, 200), 1.0);
        assert_eq!((left.edge, right.edge), (Edge::Right, Edge::Left));
        assert_eq!((left.region.x, left.region.width), (0.0, 40.0));
        assert_eq!((right.region.x, right.region.width), (60.0, 40.0));
    }

    #[test]
    fn a_gap_wider_than_the_band_keeps_two_pixels() {
        let config = config(MirrorOrientation::Horizontal, 500, |_| {});
        let ([top, bottom], _) = strips(&config, (200, 100), 1.0);
        assert_eq!(top.region.height, 1.0);
        assert_eq!(bottom.region.y - (top.region.y + top.region.height), 98.0);
    }

    #[test]
    fn full_length_spans_the_surface_and_offsets_move_the_band() {
        let config = config(MirrorOrientation::Horizontal, 0, |c| {
            c.overlay.full_length = true;
            c.visualizer.center_offset_x = 10.0;
            c.visualizer.center_offset_y = -20.0;
            c.visualizer.gradient_direction = GradientDirection::Horizontal;
        });
        let ([top, _], axis) = strips(&config, (400, 300), 1.5);
        // 150 logical tall at scale 1.5, moved up by 30
        assert_eq!((top.region.x, top.region.width), (15.0, 400.0));
        assert_eq!((top.region.y, top.region.height), (45.0, 75.0));
        // a horizontal gradient runs along the band
        assert!(axis.columns && axis.start == 15.0 && axis.length == 400.0);
    }

    #[test]
    fn a_vertical_gradient_crosses_the_band_from_its_start() {
        let config = config(MirrorOrientation::Horizontal, 0, |c| {
            c.visualizer.gradient_direction = GradientDirection::Vertical;
        });
        let (_, axis) = strips(&config, (400, 300), 1.0);
        assert!(!axis.columns && !axis.reversed);
        assert_eq!((axis.start, axis.length), (100.0, 100.0));
    }
}
