use kwybars_common::config::{FrameMirrorMode, OverlayPosition, RgbaColor, VisualizerColorMode};
use tracing::error;

use super::color::color_for_index;
use super::draw;

#[derive(Clone, Copy)]
pub(super) struct FrameMetrics {
    pub(super) width: f64,
    pub(super) height: f64,
    pub(super) top_thickness: f64,
    pub(super) bottom_thickness: f64,
    pub(super) left_thickness: f64,
    pub(super) right_thickness: f64,
    pub(super) anchor_margin: f64,
    pub(super) margin_left: f64,
    pub(super) margin_right: f64,
    pub(super) margin_top: f64,
    pub(super) margin_bottom: f64,
}

pub(super) struct EdgePaint<'a> {
    pub(super) ctx: &'a gtk::cairo::Context,
    pub(super) total_count: usize,
    pub(super) global_offset: usize,
    pub(super) style: draw::BarStyle,
    pub(super) color_mode: VisualizerColorMode,
    pub(super) color: RgbaColor,
    pub(super) color2: RgbaColor,
    pub(super) theme_colors: Option<&'a [RgbaColor]>,
}

pub(super) struct FrameEdgeSlice<'a> {
    pub(super) values: &'a [f64],
    pub(super) global_offset: usize,
}

pub(super) fn normalized_frame_edges(edges: &[OverlayPosition]) -> Vec<OverlayPosition> {
    let mut normalized = Vec::new();
    for edge in edges {
        if !normalized.contains(edge) {
            normalized.push(edge.clone());
        }
    }
    normalized
}

pub(super) fn resolve_frame_edge_slice<'a>(
    values: &'a [f64],
    active_edges: &[OverlayPosition],
    edge_index: usize,
    mirror_mode: FrameMirrorMode,
) -> FrameEdgeSlice<'a> {
    match mirror_mode {
        FrameMirrorMode::Off => FrameEdgeSlice {
            values: draw::distributed_chunk(values, edge_index, active_edges.len()),
            global_offset: values.len() * edge_index / active_edges.len(),
        },
        FrameMirrorMode::All => FrameEdgeSlice {
            values,
            global_offset: 0,
        },
        FrameMirrorMode::Pairs => {
            let has_horizontal = active_edges
                .iter()
                .any(|edge| matches!(edge, OverlayPosition::Top | OverlayPosition::Bottom));
            let has_vertical = active_edges
                .iter()
                .any(|edge| matches!(edge, OverlayPosition::Left | OverlayPosition::Right));

            if has_horizontal && has_vertical {
                let (group_index, group_offset) = if matches!(
                    active_edges[edge_index],
                    OverlayPosition::Top | OverlayPosition::Bottom
                ) {
                    (0, 0)
                } else {
                    (1, values.len() / 2)
                };

                FrameEdgeSlice {
                    values: draw::distributed_chunk(values, group_index, 2),
                    global_offset: group_offset,
                }
            } else {
                FrameEdgeSlice {
                    values,
                    global_offset: 0,
                }
            }
        }
    }
}

pub(super) fn frame_edge_rect(edge: OverlayPosition, metrics: FrameMetrics) -> draw::FrameEdgeRect {
    match edge {
        OverlayPosition::Top => draw::FrameEdgeRect {
            x: metrics.margin_left,
            y: metrics.anchor_margin + metrics.margin_top,
            width: (metrics.width - metrics.margin_left - metrics.margin_right).max(1.0),
            height: metrics.top_thickness.max(1.0),
            orientation: draw::BarOrientation::Horizontal,
            from_start: true,
        },
        OverlayPosition::Bottom => draw::FrameEdgeRect {
            x: metrics.margin_left,
            y: (metrics.height
                - metrics.anchor_margin
                - metrics.margin_bottom
                - metrics.bottom_thickness)
                .max(0.0),
            width: (metrics.width - metrics.margin_left - metrics.margin_right).max(1.0),
            height: metrics.bottom_thickness.max(1.0),
            orientation: draw::BarOrientation::Horizontal,
            from_start: false,
        },
        OverlayPosition::Left => draw::FrameEdgeRect {
            x: metrics.anchor_margin + metrics.margin_left,
            y: metrics.margin_top,
            width: metrics.left_thickness.max(1.0),
            height: (metrics.height - metrics.margin_top - metrics.margin_bottom).max(1.0),
            orientation: draw::BarOrientation::Vertical,
            from_start: true,
        },
        OverlayPosition::Right => draw::FrameEdgeRect {
            x: (metrics.width
                - metrics.anchor_margin
                - metrics.margin_right
                - metrics.right_thickness)
                .max(0.0),
            y: metrics.margin_top,
            width: metrics.right_thickness.max(1.0),
            height: (metrics.height - metrics.margin_top - metrics.margin_bottom).max(1.0),
            orientation: draw::BarOrientation::Vertical,
            from_start: false,
        },
    }
}

pub(super) fn paint_line_edge(
    values: &[f64],
    edge_rect: draw::FrameEdgeRect,
    edge_paint: &EdgePaint<'_>,
) {
    let paint_color = |ctx: &gtk::cairo::Context, local_index: usize| {
        let global_index = edge_paint.global_offset + local_index;
        let resolved = if let Some(colors) = edge_paint.theme_colors {
            let color_idx =
                draw::bar_color_index(global_index, edge_paint.total_count, colors.len());
            colors[color_idx]
        } else {
            color_for_index(
                edge_paint.color_mode,
                edge_paint.color,
                edge_paint.color2,
                global_index,
                edge_paint.total_count,
            )
        };

        ctx.set_source_rgba(
            f64::from(resolved.r),
            f64::from(resolved.g),
            f64::from(resolved.b),
            f64::from(resolved.a),
        );
    };

    match edge_rect.orientation {
        draw::BarOrientation::Horizontal => {
            draw::for_each_horizontal_bar(
                values,
                edge_rect.width,
                edge_rect.height,
                edge_paint.style.thickness,
                edge_paint.style.gap,
                edge_rect.from_start,
                |index, x, y, bar_width, bar_height| {
                    paint_color(edge_paint.ctx, index);
                    draw::append_bar_path(
                        edge_paint.ctx,
                        draw::BarRect {
                            x: edge_rect.x + x,
                            y: edge_rect.y + y,
                            width: bar_width,
                            height: bar_height,
                        },
                        edge_paint.style,
                        draw::BarOrientation::Horizontal,
                        edge_rect.from_start,
                    );
                    if edge_paint.ctx.fill().is_err() {
                        error!("kwybars: cairo fill failed");
                    }
                },
            );
        }
        draw::BarOrientation::Vertical => {
            draw::for_each_vertical_bar(
                values,
                edge_rect.width,
                edge_rect.height,
                edge_paint.style.thickness,
                edge_paint.style.gap,
                edge_rect.from_start,
                |index, x, y, bar_width, bar_height| {
                    paint_color(edge_paint.ctx, index);
                    draw::append_bar_path(
                        edge_paint.ctx,
                        draw::BarRect {
                            x: edge_rect.x + x,
                            y: edge_rect.y + y,
                            width: bar_width,
                            height: bar_height,
                        },
                        edge_paint.style,
                        draw::BarOrientation::Vertical,
                        edge_rect.from_start,
                    );
                    if edge_paint.ctx.fill().is_err() {
                        error!("kwybars: cairo fill failed");
                    }
                },
            );
        }
    }
}
