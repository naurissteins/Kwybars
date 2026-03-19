use std::f64::consts::{PI, TAU};

use super::path::centered_layout_outer_radius;
use super::types::{
    BarStyle, DirectedBarSpec, Point, PolygonLayout, RadialBarSpec, RadialDistribution,
    RadialLayout,
};

pub fn for_each_radial_bar(
    values: &[f64],
    layout: RadialLayout,
    style: BarStyle,
    mut paint: impl FnMut(usize, RadialBarSpec),
) {
    if values.is_empty() || layout.width <= 0.0 || layout.height <= 0.0 {
        return;
    }

    let min_half_extent = (layout.width * 0.5).min(layout.height * 0.5);
    let padding = style.thickness.max(2.0) + style.gap.max(0.0);
    let max_outer_radius = (min_half_extent - padding).max(10.0);
    let inner_radius = layout
        .inner_radius
        .max(10.0)
        .min((max_outer_radius - 10.0).max(10.0));
    let max_length = (max_outer_radius - inner_radius).max(6.0);

    let Some(distribution) = radial_distribution(
        values.len(),
        inner_radius,
        style.thickness,
        style.gap,
        layout.start_angle,
        layout.arc_radians,
    ) else {
        return;
    };

    for (index, value) in values.iter().enumerate() {
        let length = (value.clamp(0.0, 1.0) * max_length).max(2.0);
        let angle = distribution.first_angle + (index as f64 * distribution.angle_step);
        paint(
            index,
            RadialBarSpec {
                angle,
                inner_radius,
                length,
                thickness: distribution.tangential_thickness,
            },
        );
    }
}

pub fn for_each_polygon_bar(
    values: &[f64],
    layout: PolygonLayout,
    style: BarStyle,
    mut paint: impl FnMut(usize, DirectedBarSpec),
) {
    if values.is_empty() || layout.width <= 0.0 || layout.height <= 0.0 || layout.sides < 3 {
        return;
    }

    let max_outer_radius = centered_layout_outer_radius(layout.width, layout.height, style);
    let radius = layout
        .radius
        .max(10.0)
        .min((max_outer_radius - 10.0).max(10.0));
    let apothem = radius * (PI / layout.sides as f64).cos();
    let max_length = (max_outer_radius - apothem).max(6.0);
    let vertices = regular_polygon_vertices(layout.sides, radius, layout.rotation_radians);
    let edge_length = polygon_edge_length(&vertices);
    if edge_length <= 0.0 {
        return;
    }

    let perimeter = edge_length * layout.sides as f64;
    let gap_count = if values.len() <= 1 { 0 } else { values.len() } as f64;
    let total_nominal =
        (values.len() as f64 * style.thickness.max(1.0)) + (gap_count * style.gap.max(0.0));
    let scale = if total_nominal > perimeter {
        perimeter / total_nominal
    } else {
        1.0
    };

    let tangential_thickness = (style.thickness * scale).max(1.0);
    let base_gap = style.gap.max(0.0) * scale;
    let occupied_length = (values.len() as f64 * tangential_thickness) + (gap_count * base_gap);
    let extra_gap = if gap_count > 0.0 {
        (perimeter - occupied_length).max(0.0) / gap_count
    } else {
        0.0
    };
    let step_distance = tangential_thickness + base_gap + extra_gap;

    for (index, value) in values.iter().enumerate() {
        let center_distance = (tangential_thickness * 0.5) + (index as f64 * step_distance);
        let (point, normal) = polygon_point_and_normal(&vertices, center_distance % perimeter);
        let length = (value.clamp(0.0, 1.0) * max_length).max(2.0);
        paint(
            index,
            DirectedBarSpec {
                x: point.x,
                y: point.y,
                angle: normal.y.atan2(normal.x),
                length,
                thickness: tangential_thickness,
            },
        );
    }
}

pub(crate) fn radial_distribution(
    count: usize,
    inner_radius: f64,
    thickness: f64,
    gap: f64,
    start_angle: f64,
    arc_radians: f64,
) -> Option<RadialDistribution> {
    if count == 0 {
        return None;
    }

    let inner_radius = inner_radius.max(1.0);
    let clamped_arc = arc_radians.clamp(-TAU, TAU);
    let direction = if clamped_arc < 0.0 { -1.0 } else { 1.0 };
    let arc_magnitude = clamped_arc.abs().max(0.001);
    let full_circle = (arc_magnitude - TAU).abs() < 0.001;

    let gap_count = if count <= 1 {
        0
    } else if full_circle {
        count
    } else {
        count.saturating_sub(1)
    } as f64;
    let total_nominal = (count as f64 * thickness.max(1.0)) + (gap_count * gap.max(0.0));
    let available_arc_length = arc_magnitude * inner_radius;
    let scale = if total_nominal > available_arc_length {
        available_arc_length / total_nominal
    } else {
        1.0
    };

    let tangential_thickness = (thickness * scale).max(1.0);
    let base_gap = gap.max(0.0) * scale;
    let occupied_length = (count as f64 * tangential_thickness) + (gap_count * base_gap);
    let extra_gap = if gap_count > 0.0 {
        (available_arc_length - occupied_length).max(0.0) / gap_count
    } else {
        0.0
    };
    let effective_gap = base_gap + extra_gap;
    let angle_step = if count <= 1 {
        0.0
    } else {
        direction * (tangential_thickness + effective_gap) / inner_radius
    };
    let first_angle = if full_circle {
        start_angle
    } else if count == 1 {
        start_angle + (clamped_arc * 0.5)
    } else {
        start_angle + (direction * tangential_thickness * 0.5 / inner_radius)
    };

    Some(RadialDistribution {
        first_angle,
        angle_step,
        tangential_thickness,
    })
}

fn regular_polygon_vertices(sides: usize, radius: f64, rotation_radians: f64) -> Vec<Point> {
    (0..sides)
        .map(|index| {
            let angle = rotation_radians + (index as f64 * TAU / sides as f64);
            Point {
                x: radius * angle.cos(),
                y: radius * angle.sin(),
            }
        })
        .collect()
}

fn polygon_edge_length(vertices: &[Point]) -> f64 {
    if vertices.len() < 2 {
        return 0.0;
    }

    let first = vertices[0];
    let second = vertices[1];
    point_distance(first, second)
}

fn polygon_point_and_normal(vertices: &[Point], distance: f64) -> (Point, Point) {
    let edge_length = polygon_edge_length(vertices).max(1.0);
    let edge_index = ((distance / edge_length).floor() as usize) % vertices.len();
    let edge_start = vertices[edge_index];
    let edge_end = vertices[(edge_index + 1) % vertices.len()];
    let along = (distance % edge_length) / edge_length;
    let point = Point {
        x: edge_start.x + ((edge_end.x - edge_start.x) * along),
        y: edge_start.y + ((edge_end.y - edge_start.y) * along),
    };
    let midpoint = Point {
        x: (edge_start.x + edge_end.x) * 0.5,
        y: (edge_start.y + edge_end.y) * 0.5,
    };
    let normal = normalize_point(midpoint);
    (point, normal)
}

fn normalize_point(point: Point) -> Point {
    let length = (point.x.powi(2) + point.y.powi(2)).sqrt();
    if length <= f64::EPSILON {
        return Point { x: 1.0, y: 0.0 };
    }

    Point {
        x: point.x / length,
        y: point.y / length,
    }
}

fn point_distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}
