//! `frame`: a strip of bars along each chosen output edge, following the
//! legacy overlay

use std::ops::Range;

use super::fill::Axis;
use super::line::{Region, Strip};
use crate::config::{Edge, FrameMirrorMode, GradientDirection, LineMode, SurfaceConfig};

/// shortest a bar gets, in logical pixels, as in the legacy overlay
const MIN_EXTENT: f32 = 2.0;

/// one edge: its strip, the bar values it shows, and its gradient
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub strip: Strip,
    pub values: Range<usize>,
    pub axis: Axis,
}

/// the strips of every edge in `frame_edges` for a buffer of `size`
pub fn parts(config: &SurfaceConfig, size: (u32, u32), scale: f32, bars: usize) -> Vec<Part> {
    let visualizer = &config.visualizer;
    let mut edges: Vec<Edge> = Vec::with_capacity(4);
    for edge in &visualizer.frame_edges {
        if !edges.contains(edge) {
            edges.push(*edge);
        }
    }
    edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| {
            let values = values(visualizer.frame_mirror_mode, &edges, index, bars);
            if values.is_empty() {
                return None;
            }
            let region = region(config, *edge, size, scale);
            Some(Part {
                strip: Strip {
                    edge: *edge,
                    region,
                    min_extent: MIN_EXTENT,
                    // legacy frame edges ignore `line_mode`
                    mode: LineMode::Continuous,
                },
                values,
                axis: axis(*edge, region, visualizer.gradient_direction),
            })
        })
        .collect()
}

/// which of `bars` values edge `index` of `edges` shows
fn values(mode: FrameMirrorMode, edges: &[Edge], index: usize, bars: usize) -> Range<usize> {
    let chunk = |group: usize, groups: usize| bars * group / groups..bars * (group + 1) / groups;
    match mode {
        FrameMirrorMode::Off => chunk(index, edges.len()),
        FrameMirrorMode::All => 0..bars,
        FrameMirrorMode::Pairs => {
            let across = |edge: &Edge| matches!(edge, Edge::Top | Edge::Bottom);
            let has_across = edges.iter().any(across);
            let has_down = edges.iter().any(|edge| !across(edge));
            match edges.get(index) {
                Some(edge) if has_across && has_down => chunk(usize::from(!across(edge)), 2),
                _ => 0..bars,
            }
        }
    }
}

fn region(config: &SurfaceConfig, edge: Edge, (width, height): (u32, u32), scale: f32) -> Region {
    let overlay = &config.overlay;
    let px = |value: u32| value as f32 * scale;
    let (width, height) = (width as f32, height as f32);
    let (left, right) = (px(overlay.margin_left), px(overlay.margin_right));
    let (top, bottom) = (px(overlay.margin_top), px(overlay.margin_bottom));
    let anchor = px(overlay.anchor_margin);
    let across = px(overlay.height.max(1));
    let down = px(overlay.width.max(1));
    let span_x = (width - left - right).max(1.0);
    let span_y = (height - top - bottom).max(1.0);
    match edge {
        Edge::Top => Region {
            x: left,
            y: anchor + top,
            width: span_x,
            height: across,
        },
        Edge::Bottom => Region {
            x: left,
            y: (height - anchor - bottom - across).max(0.0),
            width: span_x,
            height: across,
        },
        Edge::Left => Region {
            x: anchor + left,
            y: top,
            width: down,
            height: span_y,
        },
        Edge::Right => Region {
            x: (width - anchor - right - down).max(0.0),
            y: top,
            width: down,
            height: span_y,
        },
    }
}

/// legacy's edge gradient: a vertical one starts at the edge's base and
/// crosses the band, a horizontal one runs along it
fn axis(edge: Edge, region: Region, direction: GradientDirection) -> Axis {
    let across = matches!(edge, Edge::Top | Edge::Bottom);
    let columns = across == (direction == GradientDirection::Horizontal);
    let (start, length) = if columns {
        (region.x, region.width)
    } else {
        (region.y, region.height)
    };
    Axis {
        columns,
        start,
        length,
        reversed: direction == GradientDirection::Vertical
            && matches!(edge, Edge::Bottom | Edge::Right),
    }
}

#[cfg(test)]
mod tests {
    use super::{parts, values};
    use crate::config::{Config, Edge, FrameMirrorMode, GradientDirection, SurfaceConfig};

    fn config(edges: &[Edge], mode: FrameMirrorMode) -> SurfaceConfig {
        let mut config = Config::default();
        config.visualizer.frame_edges = edges.to_vec();
        config.visualizer.frame_mirror_mode = mode;
        config.overlay.anchor_margin = 10;
        config.overlay.margin_left = 5;
        config.overlay.margin_right = 5;
        config.overlay.margin_top = 2;
        config.overlay.margin_bottom = 3;
        config.overlay.height = 40;
        config.overlay.width = 30;
        config.surface(None, None)
    }

    #[test]
    fn values_split_share_or_pair_by_mode() {
        let all = [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left];
        let ranges = |mode, edges: &[Edge]| {
            (0..edges.len())
                .map(|index| values(mode, edges, index, 10))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ranges(FrameMirrorMode::Off, &all),
            vec![0..2, 2..5, 5..7, 7..10]
        );
        assert_eq!(ranges(FrameMirrorMode::All, &all[..2]), vec![0..10, 0..10]);
        // top and bottom share the first half, the sides the second
        assert_eq!(
            ranges(FrameMirrorMode::Pairs, &all),
            vec![0..5, 5..10, 0..5, 5..10]
        );
        // pairs without both directions show everything on each edge
        let across = [Edge::Top, Edge::Bottom];
        assert_eq!(ranges(FrameMirrorMode::Pairs, &across), vec![0..10, 0..10]);
    }

    #[test]
    fn edges_sit_inside_the_margins() {
        let config = config(
            &[Edge::Top, Edge::Bottom, Edge::Left, Edge::Right, Edge::Top],
            FrameMirrorMode::All,
        );
        let found = parts(&config, (400, 300), 1.0, 8);
        // the repeated top edge is dropped
        assert_eq!(found.len(), 4);
        let region = |index: usize| found[index].strip.region;
        let top = region(0);
        assert_eq!(
            (top.x, top.y, top.width, top.height),
            (5.0, 12.0, 390.0, 40.0)
        );
        let bottom = region(1);
        assert_eq!((bottom.y, bottom.height), (247.0, 40.0));
        let left = region(2);
        assert_eq!(
            (left.x, left.y, left.width, left.height),
            (15.0, 2.0, 30.0, 295.0)
        );
        let right = region(3);
        assert_eq!((right.x, right.width), (355.0, 30.0));
    }

    #[test]
    fn a_vertical_gradient_starts_at_each_edge() {
        let mut config = config(&[Edge::Top, Edge::Bottom], FrameMirrorMode::All);
        config.visualizer.gradient_direction = GradientDirection::Vertical;
        let found = parts(&config, (400, 300), 1.0, 8);
        let (top, bottom) = (found[0].axis, found[1].axis);
        assert!(!top.columns && !top.reversed && top.start == 12.0);
        assert!(!bottom.columns && bottom.reversed && bottom.start == 247.0);
        config.visualizer.gradient_direction = GradientDirection::Horizontal;
        let along = parts(&config, (400, 300), 1.0, 8)[0].axis;
        assert!(along.columns && along.start == 5.0 && along.length == 390.0);
    }

    #[test]
    fn edges_without_values_are_skipped() {
        let config = config(&[Edge::Top, Edge::Bottom], FrameMirrorMode::Off);
        assert_eq!(parts(&config, (100, 100), 1.0, 1).len(), 1);
    }
}
