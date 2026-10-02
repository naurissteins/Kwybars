use super::LineLayout;
use crate::config::{Edge, LineMode, VisualizerConfig};
use crate::render::PixelRect;

fn visualizer(edit: impl FnOnce(&mut VisualizerConfig)) -> VisualizerConfig {
    let mut visualizer = VisualizerConfig {
        bar_width: 10,
        gap: 5,
        bar_corner_radius: 0.0,
        ..VisualizerConfig::default()
    };
    edit(&mut visualizer);
    visualizer
}

#[test]
fn bars_sit_on_whole_pixels_without_sharing() {
    for (length, bars, scale) in [
        (1000, 256, 1.0),
        (97, 30, 1.0),
        (2520, 50, 1.5),
        (300, 290, 1.25),
    ] {
        let layout =
            LineLayout::along(&visualizer(|_| {}), Edge::Bottom, (length, 50), scale, bars);
        let mut previous_end = 0;
        for &(start, end) in &layout.spans {
            assert!(
                start >= previous_end && end >= start,
                "{length} px, {bars} bars"
            );
            previous_end = end;
        }
    }
}

#[test]
fn split_gap_is_kept_in_pixels() {
    let config = visualizer(|v| {
        v.line_mode = LineMode::Split;
        v.line_split_gap = 80;
    });
    let layout = LineLayout::along(&config, Edge::Bottom, (400, 100), 1.0, 4);
    assert!(layout.spans[2].0 - layout.spans[1].1 >= 80);
}

#[test]
fn logical_sizes_scale_to_buffer_pixels() {
    let layout = LineLayout::along(&visualizer(|_| {}), Edge::Bottom, (300, 100), 1.5, 2);
    // two bars of 15 with a gap of 7.5, centered in 300
    assert_eq!(layout.spans, vec![(131, 146), (154, 169)]);
    assert_eq!(layout.extent(0.0), 3.0);
    assert_eq!(layout.extent(0.5), 50.0);
    assert_eq!(layout.extent(7.0), 100.0);
}

#[test]
fn bars_grow_away_from_their_edge() {
    let config = visualizer(|_| {});
    let area = |edge, size| LineLayout::along(&config, edge, size, 1.0, 1).area(0, 30.0);
    let pixels = |x, y, width, height| {
        Some(PixelRect {
            x,
            y,
            width,
            height,
        })
    };
    assert_eq!(area(Edge::Bottom, (20, 100)), pixels(5, 70, 10, 30));
    assert_eq!(area(Edge::Top, (20, 100)), pixels(5, 0, 10, 30));
    assert_eq!(area(Edge::Left, (100, 20)), pixels(0, 5, 30, 10));
    assert_eq!(area(Edge::Right, (100, 20)), pixels(70, 5, 30, 10));
}

#[test]
fn changes_cover_the_moved_tip_and_what_depends_on_it() {
    let plain = LineLayout::along(&visualizer(|_| {}), Edge::Bottom, (20, 100), 1.0, 1);
    assert_eq!(
        plain.change(0, 30.0, 40.5).map(|a| (a.y, a.height)),
        Some((59, 11))
    );
    assert_eq!(plain.change(0, 30.0, 30.0), None);

    let rounded = LineLayout::along(
        &visualizer(|v| v.bar_corner_radius = 20.0),
        Edge::Bottom,
        (20, 100),
        1.0,
        1,
    );
    // the radius is capped at half the bar width, 5: the cap below the lower tip moves too
    assert_eq!(
        rounded.change(0, 30.0, 40.0).map(|a| (a.y, a.height)),
        Some((60, 15))
    );
    // below twice the radius the bottom corners change as well
    assert_eq!(
        rounded.change(0, 8.0, 40.0).map(|a| (a.y, a.height)),
        Some((60, 40))
    );

    let segmented = LineLayout::along(
        &visualizer(|v| {
            v.segmented_bars = true;
            v.segment_length = 7;
            v.segment_gap = 3;
        }),
        Edge::Bottom,
        (20, 100),
        1.0,
        1,
    );
    // the tip is in the segment starting at 20
    assert_eq!(
        segmented.change(0, 24.0, 33.0).map(|a| (a.y, a.height)),
        Some((67, 13))
    );
}
