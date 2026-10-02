use super::DotLayout;
use crate::config::{Edge, VisualizerConfig};
use crate::render::{PixelRect, Pose};

fn visualizer(bar_width: u32, gap: u32) -> VisualizerConfig {
    VisualizerConfig {
        bar_width,
        gap,
        ..VisualizerConfig::default()
    }
}

fn pose(extent: f32, shift: f32) -> Pose {
    Pose { extent, shift }
}

#[test]
fn particles_sit_centered_in_a_row() {
    let dots = DotLayout::new(&visualizer(10, 5), Edge::Bottom, (100, 40), 1.0, 3, false);
    assert_eq!(dots.spans, vec![(15, 35), (40, 60), (65, 85)]);
    assert_eq!(dots.pose(0, 1.0, 0.0), pose(10.0, 20.0));
    assert_eq!(dots.pose(1, 0.5, 0.0), pose(5.0, 20.0));
    assert_eq!(dots.pose(1, 0.5, 1.0), pose(5.0, 20.0));
}

#[test]
fn particles_shrink_to_the_length_and_the_depth() {
    let row = DotLayout::new(&visualizer(10, 5), Edge::Top, (35, 40), 1.0, 3, false);
    assert_eq!(row.spans, vec![(0, 10), (13, 23), (25, 35)]);
    let flat = DotLayout::new(&visualizer(10, 8), Edge::Bottom, (100, 10), 1.0, 2, false);
    assert_eq!(flat.spans, vec![(38, 48), (52, 62)]);
    assert_eq!(flat.pose(0, 1.0, 0.0), pose(5.0, 5.0));
}

#[test]
fn quiet_dots_keep_a_minimum_size() {
    let dots = DotLayout::new(&visualizer(10, 5), Edge::Bottom, (100, 40), 1.5, 3, false);
    assert_eq!(dots.pose(0, 0.0, 0.0).extent, 1.5);
    let tiny = DotLayout::new(&visualizer(1, 0), Edge::Bottom, (100, 40), 0.5, 3, false);
    assert!(tiny.spans.iter().all(|(start, end)| end - start == 1));
    assert_eq!(tiny.pose(0, 1.0, 0.0).extent, 0.5);
}

#[test]
fn floating_dots_rest_at_their_edge_and_rise_away_from_it() {
    let v = visualizer(10, 5);
    let cases = [
        (Edge::Bottom, (100, 60), 50.0, 10.0),
        (Edge::Top, (100, 60), 10.0, 50.0),
        (Edge::Left, (60, 100), 10.0, 50.0),
        (Edge::Right, (60, 100), 50.0, 10.0),
    ];
    for (edge, size, rest, far) in cases {
        let dots = DotLayout::new(&v, edge, size, 1.0, 3, true);
        assert_eq!(dots.pose(2, 1.0, 0.0), pose(10.0, rest), "{edge:?}");
        assert_eq!(dots.pose(2, 1.0, 1.0), pose(10.0, far), "{edge:?}");
        assert_eq!(dots.pose(2, 1.0, 0.5).shift, 30.0, "{edge:?}");
    }
    let shallow = DotLayout::new(&v, Edge::Bottom, (100, 30), 1.0, 3, true);
    assert_eq!(shallow.pose(0, 1.0, 0.0), pose(7.5, 22.5));
}

#[test]
fn dots_stay_inside_their_own_columns() {
    for (length, bars, scale) in [(1000, 256, 1.0), (97, 30, 1.0), (2520, 50, 1.5)] {
        let dots = DotLayout::new(
            &visualizer(20, 3),
            Edge::Bottom,
            (length, 400),
            scale,
            bars,
            true,
        );
        let mut previous_end = 0;
        for (index, &(start, end)) in dots.spans.iter().enumerate() {
            assert!(start >= previous_end && end > start, "{length} px, {bars}");
            previous_end = end;
            let Some(area) = dots.area(index, dots.pose(index, 1.0, 0.7)) else {
                panic!("dot {index} has no pixels");
            };
            assert!(
                area.x >= start && area.right() <= end,
                "{area:?} {start}..{end}"
            );
        }
    }
}

#[test]
fn a_moved_dot_damages_where_it_was_and_where_it_is() {
    let dots = DotLayout::new(&visualizer(10, 5), Edge::Left, (60, 100), 1.0, 3, true);
    let (low, high) = (dots.pose(0, 0.5, 0.0), dots.pose(0, 1.0, 1.0));
    let cover = dots.cover(0, low, high);
    assert_eq!(
        cover,
        Some(PixelRect {
            x: 5,
            y: 15,
            width: 55,
            height: 20,
        })
    );
    assert_eq!(dots.cover(0, low, low), dots.area(0, low));
}
