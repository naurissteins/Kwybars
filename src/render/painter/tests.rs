use super::{BufferContents, Painter};
use crate::config::{Config, Edge, Rgba};
use crate::render::{ByteOrder, Canvas, PixelRect};

const SIZE: (u32, u32) = (40, 20);

fn painter(bars: usize) -> Painter {
    let mut config = Config::default();
    config.visualizer.color_rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);
    Painter::new(&config.surface(None, None), bars, SIZE, ByteOrder::Rgba)
}

fn column_height(data: &[u8], x: u32) -> u32 {
    (0..SIZE.1)
        .filter(|y| data[((y * SIZE.0 + x) * 4 + 3) as usize] > 0)
        .count() as u32
}

fn paint(painter: &Painter, data: &mut [u8], contents: &mut BufferContents) {
    let Some(mut canvas) = Canvas::new(data, SIZE) else {
        panic!("canvas");
    };
    painter.paint(&mut canvas, contents);
}

#[test]
fn a_reused_buffer_is_patched_to_the_new_frame() {
    let mut painter = painter(4);
    let mut data = vec![0xAA_u8; (SIZE.0 * SIZE.1 * 4) as usize];
    let mut contents = painter.new_contents();

    painter.layout(&[1.0, 0.5, 0.0, 0.25]);
    paint(&painter, &mut data, &mut contents);
    // stale bytes from an unknown buffer are cleared
    assert_eq!(column_height(&data, 0), 0);
    assert_eq!(
        [2, 12, 22, 32].map(|x| column_height(&data, x)),
        [20, 10, 0, 5]
    );

    painter.layout(&[0.25, 0.5, 1.0, 0.0]);
    paint(&painter, &mut data, &mut contents);
    assert_eq!(
        [2, 12, 22, 32].map(|x| column_height(&data, x)),
        [5, 10, 20, 0]
    );
}

#[test]
fn damage_covers_only_the_bars_that_moved() {
    let mut painter = painter(4);
    painter.layout(&[1.0, 0.5, 0.0, 0.25]);
    let mut areas = Vec::new();
    painter.present(|area| areas.push(area));
    assert_eq!(areas, vec![PixelRect::full(SIZE)]);

    assert!(painter.layout(&[1.0, 0.75, 0.0, 0.25]));
    areas.clear();
    painter.present(|area| areas.push(area));
    assert_eq!(
        areas,
        vec![PixelRect {
            x: 12,
            y: 5,
            width: 7,
            height: 5,
        }]
    );
    assert!(!painter.layout(&[1.0, 0.75, 0.0, 0.25]));
}

#[test]
fn a_new_size_is_damaged_whole() {
    let mut painter = painter(2);
    painter.layout(&[0.5, 0.5]);
    painter.present(|_| {});
    painter.resize((20, 10), Edge::Bottom);
    assert!(painter.layout(&[0.5, 0.5]));
    let mut areas = Vec::new();
    painter.present(|area| areas.push(area));
    assert_eq!(areas, vec![PixelRect::full((20, 10))]);
}

#[test]
fn painting_does_not_reallocate() {
    let mut painter = painter(8);
    let mut data = vec![0_u8; (SIZE.0 * SIZE.1 * 4) as usize];
    let mut contents = painter.new_contents();
    let before = (painter.next.as_ptr(), painter.shown.as_ptr());
    for step in 0..50 {
        let value = step as f32 / 50.0;
        painter.layout(&[value; 8]);
        paint(&painter, &mut data, &mut contents);
        painter.present(|_| {});
    }
    assert_eq!(before, (painter.next.as_ptr(), painter.shown.as_ptr()));
}

#[test]
fn patched_translucent_bars_match_a_fresh_paint() {
    let mut config = Config::default();
    config.visualizer.color_rgba = Rgba::new(0.2, 0.6, 1.0, 0.5);
    let surface = config.surface(None, None);
    let mut patched = Painter::new(&surface, 4, SIZE, ByteOrder::Rgba);
    let mut data = vec![0_u8; (SIZE.0 * SIZE.1 * 4) as usize];
    let mut contents = patched.new_contents();
    let frames = [
        [0.3, 0.9, 0.0, 0.47],
        [0.55, 0.2, 0.61, 0.47],
        [0.1, 0.33, 0.9, 0.05],
        [0.12, 0.0, 0.88, 0.5],
    ];
    for heights in frames {
        patched.layout(&heights);
        paint(&patched, &mut data, &mut contents);

        let mut fresh = Painter::new(&surface, 4, SIZE, ByteOrder::Rgba);
        fresh.layout(&heights);
        let mut expected = vec![0_u8; data.len()];
        paint(&fresh, &mut expected, &mut fresh.new_contents());
        assert!(data == expected, "patched pixels differ for {heights:?}");
    }
}

#[test]
fn bgra_buffers_get_red_and_blue_swapped() {
    let mut config = Config::default();
    config.visualizer.color_rgba = Rgba::new(1.0, 0.5, 0.0, 1.0);
    let surface = config.surface(None, None);
    for (order, expected) in [
        (ByteOrder::Rgba, [255, 128, 0, 255]),
        (ByteOrder::Bgra, [0, 128, 255, 255]),
    ] {
        let mut painter = Painter::new(&surface, 1, SIZE, order);
        painter.layout(&[1.0]);
        let mut data = vec![0_u8; (SIZE.0 * SIZE.1 * 4) as usize];
        paint(&painter, &mut data, &mut painter.new_contents());
        let at = ((10 * SIZE.0 + 20) * 4) as usize;
        assert_eq!(data[at..at + 4], expected, "{order:?}");
    }
}
