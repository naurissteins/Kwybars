use super::{Axis, Fill, bar_color_index, gradient};
use crate::config::{ColorMode, Config, Edge, GradientDirection, Rgba, SurfaceConfig};
use crate::render::ByteOrder;

const RED: Rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);
const BLUE: Rgba = Rgba::new(0.0, 0.0, 1.0, 1.0);

fn surface(mode: ColorMode, direction: GradientDirection, theme: bool) -> SurfaceConfig {
    let mut config = Config::default();
    config.visualizer.color_mode = mode;
    config.visualizer.gradient_direction = direction;
    config.visualizer.color_rgba = RED;
    config.visualizer.color2_rgba = BLUE;
    let mut surface = config.surface(None, None);
    if theme {
        surface.theme_colors = Some([RED, RED, RED, BLUE, BLUE, BLUE]);
    }
    surface
}

#[test]
fn spreads_colors_evenly() {
    let indices: Vec<usize> = (0..12).map(|index| bar_color_index(index, 12, 6)).collect();
    assert_eq!(indices, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
}

#[test]
fn gradient_interpolates_between_even_stops() {
    let mid = gradient(&[RED, BLUE], 0.5);
    assert_eq!((mid.r, mid.b), (0.5, 0.5));
    assert_eq!(gradient(&[RED, BLUE, RED], 0.5), BLUE);
    assert_eq!(gradient(&[RED, BLUE], 7.0), BLUE);
}

#[test]
fn solid_mode_is_one_color() {
    let fill = Fill::new(
        &surface(ColorMode::Solid, GradientDirection::Vertical, false),
        Axis::along(
            Edge::Bottom,
            surface(ColorMode::Solid, GradientDirection::Vertical, false)
                .visualizer
                .gradient_direction,
            (10, 10),
        ),
        (10, 10),
        4,
        ByteOrder::Rgba,
    );
    assert_eq!(fill, Fill::Solid([255, 0, 0, 255]));
}

#[test]
fn vertical_gradient_starts_at_the_bars_base() {
    let config = surface(ColorMode::Gradient, GradientDirection::Vertical, false);
    let bottom = Fill::new(
        &config,
        Axis::along(Edge::Bottom, config.visualizer.gradient_direction, (10, 4)),
        (10, 4),
        4,
        ByteOrder::Rgba,
    );
    // rows from the top: mostly blue at the tip side, mostly red at the base
    assert!(matches!(&bottom, Fill::Rows(rows) if rows[0][2] > 200 && rows[3][0] > 200));
    let top = Fill::new(
        &config,
        Axis::along(Edge::Top, config.visualizer.gradient_direction, (10, 4)),
        (10, 4),
        4,
        ByteOrder::Rgba,
    );
    assert!(matches!(&top, Fill::Rows(rows) if rows[0][0] > 200 && rows[3][2] > 200));
    let right = Fill::new(
        &config,
        Axis::along(Edge::Right, config.visualizer.gradient_direction, (4, 10)),
        (4, 10),
        4,
        ByteOrder::Rgba,
    );
    assert!(matches!(&right, Fill::Columns(cols) if cols[3][0] > 200));
}

#[test]
fn horizontal_gradient_runs_along_the_bars() {
    let config = surface(ColorMode::Gradient, GradientDirection::Horizontal, true);
    let bottom = Fill::new(
        &config,
        Axis::along(Edge::Bottom, config.visualizer.gradient_direction, (6, 3)),
        (6, 3),
        4,
        ByteOrder::Rgba,
    );
    assert!(
        matches!(&bottom, Fill::Columns(cols) if cols.len() == 6 && cols[0][0] == 255 && cols[5][2] == 255)
    );
    let left = Fill::new(
        &config,
        Axis::along(Edge::Left, config.visualizer.gradient_direction, (3, 6)),
        (3, 6),
        4,
        ByteOrder::Rgba,
    );
    assert!(matches!(&left, Fill::Rows(rows) if rows.len() == 6));
}

#[test]
fn fading_scales_every_premultiplied_channel() {
    let config = surface(ColorMode::Gradient, GradientDirection::Horizontal, false);
    let base = Fill::new(
        &config,
        Axis::along(Edge::Bottom, config.visualizer.gradient_direction, (4, 4)),
        (4, 4),
        4,
        ByteOrder::Rgba,
    );
    let mut faded = base.clone();
    faded.fade_from(&base, 128);
    let (Fill::Columns(base_colors), Fill::Columns(colors)) = (&base, &faded) else {
        panic!("expected columns");
    };
    assert_eq!(base_colors[0], [223, 0, 32, 255]);
    assert_eq!(colors[0], [112, 0, 16, 128]);
    faded.fade_from(&base, 255);
    assert_eq!(faded, base);
    faded.fade_from(&base, 0);
    assert_eq!(faded.color(0, 0, 0), [0; 4]);
}

#[test]
fn dots_take_one_color_each() {
    let gradient = surface(ColorMode::Gradient, GradientDirection::Horizontal, false);
    let Fill::Bars(colors) = Fill::per_bar(&gradient, 3, ByteOrder::Rgba, false) else {
        panic!("expected a color per dot");
    };
    assert_eq!(
        colors,
        vec![[255, 0, 0, 255], [128, 0, 128, 255], [0, 0, 255, 255]]
    );
    let solid = surface(ColorMode::Solid, GradientDirection::Horizontal, false);
    assert_eq!(
        Fill::per_bar(&solid, 3, ByteOrder::Rgba, false),
        Fill::Solid([255, 0, 0, 255])
    );
    // a theme ignores the gradient direction
    let themed = surface(ColorMode::Solid, GradientDirection::Horizontal, true);
    let (red, blue) = ([255, 0, 0, 255], [0, 0, 255, 255]);
    assert_eq!(
        Fill::per_bar(&themed, 4, ByteOrder::Rgba, false),
        Fill::Bars(vec![red, red, blue, blue])
    );
    let Fill::Bars(smooth) = Fill::per_bar(&themed, 3, ByteOrder::Rgba, true) else {
        panic!("expected a color per bar");
    };
    assert_eq!(smooth, vec![red, [128, 0, 128, 255], blue]);
}

#[test]
fn theme_with_vertical_gradient_colors_whole_bars() {
    let config = surface(ColorMode::Gradient, GradientDirection::Vertical, true);
    let fill = Fill::new(
        &config,
        Axis::along(Edge::Bottom, config.visualizer.gradient_direction, (10, 10)),
        (10, 10),
        4,
        ByteOrder::Bgra,
    );
    let blue_bgra = [255, 0, 0, 255];
    let red_bgra = [0, 0, 255, 255];
    assert_eq!(
        fill,
        Fill::Bars(vec![red_bgra, red_bgra, blue_bgra, blue_bgra])
    );
    assert_eq!(fill.color(3, 0, 0), blue_bgra);
}
