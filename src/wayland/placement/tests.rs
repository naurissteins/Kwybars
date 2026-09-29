use super::{Anchors, Margins, Placement};
use crate::config::{
    Edge, HorizontalAlignment, Layer, Layout, OverlayConfig, OverlaySettings, VerticalAlignment,
};

fn defaults() -> OverlaySettings {
    OverlaySettings::from(&OverlayConfig::default())
}

fn anchors(top: bool, bottom: bool, left: bool, right: bool) -> Anchors {
    Anchors {
        top,
        bottom,
        left,
        right,
    }
}

#[test]
fn default_is_a_full_length_bar_on_the_bottom_edge() {
    let placement = Placement::new(&defaults(), Layout::Line);
    assert_eq!(
        placement,
        Placement {
            layer: Layer::Background,
            anchors: anchors(false, true, true, true),
            margins: Margins {
                top: 0,
                right: 20,
                bottom: 20,
                left: 20,
            },
            width: 0,
            height: 500,
        }
    );
}

#[test]
fn fixed_length_top_right() {
    let mut overlay = defaults();
    overlay.position = Edge::Top;
    overlay.layer = Layer::Top;
    overlay.full_length = false;
    overlay.width = 420;
    overlay.height = 160;
    overlay.horizontal_alignment = HorizontalAlignment::Right;
    overlay.margin_right = 32;
    overlay.anchor_margin = 18;
    let placement = Placement::new(&overlay, Layout::Line);
    assert_eq!(placement.layer, Layer::Top);
    assert_eq!(placement.anchors, anchors(true, false, false, true));
    assert_eq!(
        placement.margins,
        Margins {
            top: 18,
            right: 32,
            bottom: 0,
            left: 0,
        }
    );
    assert_eq!((placement.width, placement.height), (420, 160));
}

#[test]
fn full_length_left_edge_spans_vertically() {
    let mut overlay = defaults();
    overlay.position = Edge::Left;
    overlay.width = 72;
    overlay.margin_top = 12;
    overlay.margin_bottom = 16;
    let placement = Placement::new(&overlay, Layout::Line);
    assert_eq!(placement.anchors, anchors(true, true, true, false));
    assert_eq!(
        placement.margins,
        Margins {
            top: 12,
            right: 0,
            bottom: 16,
            left: 20,
        }
    );
    assert_eq!((placement.width, placement.height), (72, 0));
}

#[test]
fn centered_fixed_length_right_edge_has_only_the_edge_anchor() {
    let mut overlay = defaults();
    overlay.position = Edge::Right;
    overlay.full_length = false;
    overlay.width = 64;
    overlay.height = 280;
    overlay.vertical_alignment = VerticalAlignment::Center;
    overlay.margin_top = 99;
    let placement = Placement::new(&overlay, Layout::Line);
    assert_eq!(placement.anchors, anchors(false, false, false, true));
    assert_eq!(placement.margins.top, 0);
    assert_eq!((placement.width, placement.height), (64, 280));
}

#[test]
fn shape_layouts_fill_the_output_inside_the_margins() {
    let mut overlay = defaults();
    (overlay.margin_top, overlay.margin_right) = (10, 11);
    (overlay.margin_bottom, overlay.margin_left) = (12, 13);
    for layout in [
        Layout::Mirror,
        Layout::Frame,
        Layout::Radial,
        Layout::Polygon,
    ] {
        let placement = Placement::new(&overlay, layout);
        assert_eq!(placement.anchors, anchors(true, true, true, true));
        assert_eq!(
            placement.margins,
            Margins {
                top: 10,
                right: 11,
                bottom: 12,
                left: 13,
            }
        );
        assert_eq!((placement.width, placement.height), (0, 0));
    }
    for layout in [Layout::Wave, Layout::Particle, Layout::Floating] {
        assert_eq!(Placement::new(&overlay, layout).height, 500);
    }
}

#[test]
fn zero_size_is_only_requested_between_two_anchors() {
    let edges = [Edge::Bottom, Edge::Top, Edge::Left, Edge::Right];
    let horizontal = [
        HorizontalAlignment::Left,
        HorizontalAlignment::Center,
        HorizontalAlignment::Right,
    ];
    let vertical = [
        VerticalAlignment::Top,
        VerticalAlignment::Center,
        VerticalAlignment::Bottom,
    ];
    let mut overlay = defaults();
    overlay.width = 0;
    overlay.height = 0;
    for edge in edges {
        for full_length in [true, false] {
            for (h, v) in horizontal.into_iter().zip(vertical) {
                overlay.position = edge;
                overlay.full_length = full_length;
                overlay.horizontal_alignment = h;
                overlay.vertical_alignment = v;
                let placement = Placement::new(&overlay, Layout::Line);
                let a = placement.anchors;
                assert!(placement.width > 0 || (a.left && a.right), "{placement:?}");
                assert!(placement.height > 0 || (a.top && a.bottom), "{placement:?}");
            }
        }
    }
}

#[test]
fn huge_margins_saturate() {
    let mut overlay = defaults();
    overlay.anchor_margin = u32::MAX;
    assert_eq!(
        Placement::new(&overlay, Layout::Line).margins.bottom,
        i32::MAX
    );
}

#[test]
fn surface_size_prefers_the_compositor_then_the_request_then_the_output() {
    let placement = Placement::new(&defaults(), Layout::Line);
    assert_eq!(
        placement.surface_size((1876, 500), Some((1920, 1080))),
        (1876, 500)
    );
    assert_eq!(
        placement.surface_size((0, 0), Some((1920, 1080))),
        (1880, 500)
    );
    assert_eq!(placement.surface_size((0, 0), None), (1, 500));
    let mut overlay = defaults();
    overlay.margin_left = 5_000;
    let narrow = Placement::new(&overlay, Layout::Line);
    assert_eq!(narrow.surface_size((0, 0), Some((1920, 1080))), (1, 500));
}
