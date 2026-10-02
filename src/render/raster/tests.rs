use super::{RoundedRect, blend, edge_coverage, fill, overlap};
use crate::render::fill::Fill;
use crate::render::{Canvas, PixelRect};

const WHITE: Fill = Fill::Solid([255; 4]);

fn render(shape: RoundedRect, size: (u32, u32), clip: PixelRect) -> Vec<u8> {
    let mut data = vec![0_u8; (size.0 * size.1 * 4) as usize];
    let Some(mut canvas) = Canvas::new(&mut data, size) else {
        panic!("canvas");
    };
    fill(&mut canvas, shape, clip, &WHITE, 0);
    data
}

fn alpha(data: &[u8], width: u32, x: u32, y: u32) -> u8 {
    data[((y * width + x) * 4 + 3) as usize]
}

fn rect(left: f32, top: f32, right: f32, bottom: f32, radius: f32) -> RoundedRect {
    RoundedRect {
        left,
        top,
        right,
        bottom,
        radius,
    }
}

#[test]
fn square_rect_fills_whole_pixels_and_blends_a_fractional_edge() {
    let size = (6, 6);
    let data = render(rect(1.0, 2.5, 4.0, 6.0, 0.0), size, PixelRect::full(size));
    assert_eq!(alpha(&data, 6, 0, 4), 0);
    assert_eq!(alpha(&data, 6, 1, 4), 255);
    assert_eq!(alpha(&data, 6, 3, 5), 255);
    assert_eq!(alpha(&data, 6, 4, 5), 0);
    // half covered top row
    assert_eq!(alpha(&data, 6, 2, 2), 128);
    assert_eq!(alpha(&data, 6, 2, 1), 0);
}

#[test]
fn rounded_corners_fade_out_symmetrically() {
    let size = (20, 20);
    let data = render(rect(0.0, 0.0, 20.0, 20.0, 8.0), size, PixelRect::full(size));
    // corners are empty, the middle and the edge midpoints are full
    assert_eq!(alpha(&data, 20, 0, 0), 0);
    assert_eq!(alpha(&data, 20, 19, 19), 0);
    assert_eq!(alpha(&data, 20, 10, 10), 255);
    assert_eq!(alpha(&data, 20, 10, 0), 255);
    assert_eq!(alpha(&data, 20, 0, 10), 255);
    for (x, y) in [(2, 2), (1, 4), (4, 1)] {
        let corner = alpha(&data, 20, x, y);
        assert_eq!(corner, alpha(&data, 20, 19 - x, y));
        assert_eq!(corner, alpha(&data, 20, x, 19 - y));
        assert_eq!(corner, alpha(&data, 20, y, x));
    }
    assert!(alpha(&data, 20, 2, 2) > 0 && alpha(&data, 20, 2, 2) < 255);
}

fn reference(shape: RoundedRect, size: (u32, u32)) -> Vec<u8> {
    let mut data = vec![0_u8; (size.0 * size.1 * 4) as usize];
    let radius = shape
        .radius
        .min((shape.right - shape.left) * 0.5)
        .min((shape.bottom - shape.top) * 0.5);
    for y in 0..size.1 {
        let cy = overlap(y as f32, shape.top, shape.bottom);
        for x in 0..size.0 {
            let coverage = edge_coverage(&shape, radius, x as f32, y as f32, cy);
            let at = ((y * size.0 + x) * 4) as usize;
            let mut pixel = [0; 4];
            blend(&mut pixel, [255; 4], coverage);
            data[at..at + 4].copy_from_slice(&pixel);
        }
    }
    data
}

#[test]
fn rows_match_sampling_every_pixel() {
    let size = (40, 40);
    let shapes = [
        // circles, whole and at sub-pixel offsets
        rect(2.0, 3.0, 32.0, 33.0, 15.0),
        rect(4.3, 5.7, 25.1, 26.5, 10.4),
        rect(10.5, 10.5, 12.5, 12.5, 1.0),
        rect(7.2, 7.9, 8.0, 8.7, 0.4),
        // rounded bars, tall and short
        rect(3.0, 1.25, 17.0, 38.0, 7.0),
        rect(20.4, 30.6, 35.9, 36.1, 6.0),
        rect(1.0, 1.0, 39.0, 20.0, 3.3),
    ];
    for shape in shapes {
        let drawn = render(shape, size, PixelRect::full(size));
        let expected = reference(shape, size);
        for (index, (a, b)) in drawn.iter().zip(&expected).enumerate() {
            let pixel = (index / 4) as u32;
            assert!(
                a.abs_diff(*b) <= 1,
                "{shape:?} at {}, {}: {a} vs {b}",
                pixel % size.0,
                pixel / size.0
            );
        }
    }
}

#[test]
fn clip_limits_the_written_rows() {
    let size = (4, 8);
    let clip = PixelRect {
        x: 0,
        y: 2,
        width: 4,
        height: 3,
    };
    let data = render(rect(0.0, 0.0, 4.0, 8.0, 0.0), size, clip);
    let rows: Vec<u8> = (0..8).map(|y| alpha(&data, 4, 1, y)).collect();
    assert_eq!(rows, vec![0, 0, 255, 255, 255, 0, 0, 0]);
}

#[test]
fn a_clip_beside_the_shape_draws_nothing() {
    let size = (8, 4);
    let clip = PixelRect {
        x: 6,
        y: 0,
        width: 2,
        height: 4,
    };
    let data = render(rect(0.0, 0.0, 3.0, 4.0, 1.0), size, clip);
    assert!(data.iter().all(|byte| *byte == 0));
}

#[test]
fn touching_shapes_add_up_to_full_coverage() {
    let size = (2, 4);
    let mut data = vec![0_u8; 2 * 4 * 4];
    let Some(mut canvas) = Canvas::new(&mut data, size) else {
        panic!("canvas");
    };
    fill(
        &mut canvas,
        rect(0.0, 0.0, 2.0, 1.5, 0.0),
        PixelRect::full(size),
        &WHITE,
        0,
    );
    fill(
        &mut canvas,
        rect(0.0, 1.5, 2.0, 4.0, 0.0),
        PixelRect::full(size),
        &WHITE,
        0,
    );
    assert_eq!(alpha(&data, 2, 0, 1), 255);
}

mod turned {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

    use super::{WHITE, alpha, rect, render};
    use crate::render::raster::{Turned, clear_turned, fill_turned};
    use crate::render::{Canvas, PixelRect};

    fn turned(center: (f32, f32), angle: f32, length: f32, width: f32, radius: f32) -> Turned {
        let axis = (angle.cos(), angle.sin());
        Turned {
            start: (
                center.0 - axis.0 * length * 0.5,
                center.1 - axis.1 * length * 0.5,
            ),
            axis,
            length,
            half_width: width * 0.5,
            radius,
        }
    }

    fn render_turned(shape: Turned, size: (u32, u32), clip: PixelRect) -> Vec<u8> {
        let mut data = vec![0_u8; (size.0 * size.1 * 4) as usize];
        let Some(mut canvas) = Canvas::new(&mut data, size) else {
            panic!("canvas");
        };
        fill_turned(&mut canvas, shape, clip, &WHITE, 0);
        data
    }

    fn total_alpha(data: &[u8]) -> f32 {
        data.chunks(4)
            .map(|pixel| f32::from(pixel[3]) / 255.0)
            .sum()
    }

    #[test]
    fn unturned_matches_the_straight_rasterizer() {
        let size = (40, 40);
        for (left, top, right, bottom, radius) in [
            (3.0, 1.25, 17.0, 38.0, 7.0),
            (4.3, 5.7, 25.1, 26.5, 10.4),
            (10.2, 3.0, 10.9, 30.0, 0.0),
            (1.0, 1.0, 39.0, 20.0, 3.3),
        ] {
            let straight = render(
                rect(left, top, right, bottom, radius),
                size,
                PixelRect::full(size),
            );
            let center = ((left + right) * 0.5, (top + bottom) * 0.5);
            // along x, and along y with length and width swapped
            let along_x = turned(center, 0.0, right - left, bottom - top, radius);
            let along_y = turned(center, FRAC_PI_2, bottom - top, right - left, radius);
            for shape in [along_x, along_y] {
                let drawn = render_turned(shape, size, PixelRect::full(size));
                for (index, (a, b)) in drawn.iter().zip(&straight).enumerate() {
                    assert!(a.abs_diff(*b) <= 1, "{shape:?} byte {index}: {a} vs {b}");
                }
            }
        }
    }

    #[test]
    fn coverage_adds_up_to_the_area_at_any_angle() {
        let size = (80, 80);
        for step in 0..24 {
            let angle = step as f32 * 0.27;
            for (length, width, radius) in [(50.0, 9.0, 0.0), (44.0, 12.0, 6.0), (30.0, 1.5, 0.0)] {
                let shape = turned((40.3, 39.6), angle, length, width, radius);
                let area = length * width - (4.0 - std::f32::consts::PI) * radius * radius;
                let drawn = total_alpha(&render_turned(shape, size, PixelRect::full(size)));
                assert!(
                    (drawn - area).abs() < 0.02 * area + 1.0,
                    "{angle} rad, {length}x{width} r{radius}: {drawn} vs {area}"
                );
            }
        }
    }

    #[test]
    fn a_diagonal_bar_is_symmetric_and_solid_inside() {
        let size = (41, 41);
        let shape = turned((20.5, 20.5), FRAC_PI_4, 30.0, 8.0, 4.0);
        let data = render_turned(shape, size, PixelRect::full(size));
        assert_eq!(alpha(&data, 41, 20, 20), 255);
        assert_eq!(alpha(&data, 41, 0, 40), 0);
        for (x, y) in [(10, 10), (30, 30), (14, 12), (25, 27), (18, 23)] {
            // mirrored across the bar's axis and across its middle
            assert_eq!(alpha(&data, 41, x, y), alpha(&data, 41, y, x), "{x}, {y}");
            assert_eq!(
                alpha(&data, 41, x, y),
                alpha(&data, 41, 40 - x, 40 - y),
                "{x}, {y}"
            );
        }
    }

    #[test]
    fn clipped_pieces_add_up_to_the_whole() {
        let size = (48, 48);
        let shape = turned((23.7, 24.2), 0.6, 40.0, 10.0, 5.0);
        let whole = render_turned(shape, size, PixelRect::full(size));
        let mut data = vec![0_u8; whole.len()];
        let Some(mut canvas) = Canvas::new(&mut data, size) else {
            panic!("canvas");
        };
        for (x, y) in [(0, 0), (20, 0), (0, 17), (20, 17)] {
            let clip = PixelRect {
                x,
                y,
                width: if x == 0 { 20 } else { 28 },
                height: if y == 0 { 17 } else { 31 },
            };
            fill_turned(&mut canvas, shape, clip, &WHITE, 0);
        }
        assert!(data == whole);
    }

    #[test]
    fn rows_match_sampling_every_pixel() {
        use crate::render::raster::turned::coverage;
        let size = (60, 60);
        for step in 0..16 {
            let angle = step as f32 * 0.41;
            for (length, width, radius) in [(44.0, 9.0, 4.5), (30.0, 14.0, 3.0), (20.0, 0.8, 0.0)] {
                let shape = turned((29.8, 30.4), angle, length, width, radius);
                let drawn = render_turned(shape, size, PixelRect::full(size));
                for y in 0..size.1 {
                    for x in 0..size.0 {
                        let (dx, dy) = (
                            x as f32 + 0.5 - shape.start.0,
                            y as f32 + 0.5 - shape.start.1,
                        );
                        let (u, v) = (
                            dx * angle.cos() + dy * angle.sin(),
                            dy * angle.cos() - dx * angle.sin(),
                        );
                        let expected = (coverage(u, v, &shape) * 255.0).round();
                        let got = f32::from(alpha(&drawn, size.0, x, y));
                        assert!(
                            (got - expected).abs() <= 1.0,
                            "{angle} rad {length}x{width} at {x}, {y}: {got} vs {expected}"
                        );
                    }
                }
            }
        }
    }

    /// whether the point u along from the start, v across, is inside
    fn inside(u: f32, v: f32, length: f32, hv: f32, radius: f32) -> bool {
        if u < 0.0 || u > length || v.abs() > hv {
            return false;
        }
        let du = (radius - u).max(u - (length - radius));
        let dv = v.abs() - (hv - radius);
        du <= 0.0 || dv <= 0.0 || du * du + dv * dv <= radius * radius
    }

    /// `KWYBARS_RASTER_DIR=/tmp/x cargo test -- --ignored dumps_turned_bars`
    #[test]
    #[ignore = "needs KWYBARS_RASTER_DIR"]
    fn dumps_turned_bars_for_comparison() {
        let Some(dir) = std::env::var_os("KWYBARS_RASTER_DIR") else {
            panic!("set KWYBARS_RASTER_DIR to an empty directory");
        };
        let dir = std::path::PathBuf::from(dir);
        let size = (200, 200);
        let mut manifest = String::new();
        for (scale, radius) in [(1.0, 0.0), (1.0, 5.0), (1.5, 0.0), (1.5, 7.5)] {
            for degrees in [0.0_f32, 3.0, 10.0, 22.5, 30.0, 45.0, 60.0, 77.0] {
                let (length, width) = (100.0 * scale, 10.0 * scale);
                let angle = degrees.to_radians();
                let shape = turned((100.37, 100.61), angle, length, width, radius);
                let ours = render_turned(shape, size, PixelRect::full(size));
                let (cos, sin) = shape.axis;
                let mut truth = Vec::new();
                for y in 0..size.1 {
                    for x in 0..size.0 {
                        // 32 x 32 samples, each row staggered so no edge lines up
                        // with the grid
                        let mut hits = 0;
                        for sy in 0..32 {
                            let stagger = (sy as f32 * 0.618_034).fract();
                            for sx in 0..32 {
                                let px = x as f32 + (sx as f32 + stagger) / 32.0 - shape.start.0;
                                let py = y as f32 + (sy as f32 + 0.5) / 32.0 - shape.start.1;
                                let (u, v) = (px * cos + py * sin, py * cos - px * sin);
                                hits += u32::from(inside(u, v, length, width * 0.5, radius));
                            }
                        }
                        truth.push(((hits * 255 + 512) / 1024) as u8);
                    }
                }
                let name = format!("s{scale}-r{radius}-a{degrees}");
                let alphas: Vec<u8> = ours.chunks(4).map(|pixel| pixel[3]).collect();
                let written = std::fs::write(dir.join(format!("{name}-ours.raw")), alphas)
                    .and_then(|()| std::fs::write(dir.join(format!("{name}-truth.raw")), truth));
                assert!(written.is_ok(), "{written:?}");
                manifest.push_str(&format!(
                    "{name} {} {} {angle} {length} {} {radius}\n",
                    shape.start.0,
                    shape.start.1,
                    width * 0.5
                ));
            }
        }
        let written = std::fs::write(dir.join("manifest.txt"), manifest);
        assert!(written.is_ok(), "{written:?}");
    }

    #[test]
    fn clearing_a_shape_removes_every_pixel_it_drew() {
        let size = (60, 60);
        for step in 0..12 {
            let shape = turned((30.2, 29.7), step as f32 * 0.37, 40.0, 7.0, 3.0);
            let mut data = render_turned(shape, size, PixelRect::full(size));
            let Some(mut canvas) = Canvas::new(&mut data, size) else {
                panic!("canvas");
            };
            clear_turned(&mut canvas, shape);
            assert!(data.iter().all(|byte| *byte == 0), "step {step}");
        }
    }
}
