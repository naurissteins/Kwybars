mod draw;
mod layer;
mod style;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use kwybars_common::config::{
    AppConfig, FrameMirrorMode, OverlayPosition, RgbaColor, VisualizerColorMode, VisualizerLayout,
};
use kwybars_engine::live::LiveFrameStream;
use tracing::{error, info};

use crate::theme::ThemePalette;

#[derive(Clone, Copy)]
struct FrameMetrics {
    width: f64,
    height: f64,
    top_thickness: f64,
    bottom_thickness: f64,
    left_thickness: f64,
    right_thickness: f64,
    anchor_margin: f64,
    margin_left: f64,
    margin_right: f64,
    margin_top: f64,
    margin_bottom: f64,
}

struct EdgePaint<'a> {
    ctx: &'a gtk::cairo::Context,
    total_count: usize,
    global_offset: usize,
    style: draw::BarStyle,
    color_mode: VisualizerColorMode,
    color: RgbaColor,
    color2: RgbaColor,
    theme_colors: Option<&'a [RgbaColor]>,
}

struct FrameEdgeSlice<'a> {
    values: &'a [f64],
    global_offset: usize,
}

pub fn spawn_frame_stream(config: &AppConfig) -> Rc<LiveFrameStream> {
    let stream = Rc::new(LiveFrameStream::spawn(config.visualizer.clone()));
    info!("kwybars: using {:?} frame source", stream.source_kind());
    stream
}

pub fn build_overlay_windows(
    app: &gtk::Application,
    config: AppConfig,
    theme_palette: Option<ThemePalette>,
    stream: Rc<LiveFrameStream>,
) -> Vec<gtk::ApplicationWindow> {
    style::install_css();

    let monitors = layer::selected_monitors(&config.overlay);
    if monitors.is_empty() {
        return vec![build_overlay_window(
            app,
            &config,
            theme_palette.clone(),
            Rc::clone(&stream),
            None,
        )];
    }

    monitors
        .into_iter()
        .map(|monitor| {
            build_overlay_window(
                app,
                &config,
                theme_palette.clone(),
                Rc::clone(&stream),
                Some(monitor),
            )
        })
        .collect()
}

fn build_overlay_window(
    app: &gtk::Application,
    config: &AppConfig,
    theme_palette: Option<ThemePalette>,
    stream: Rc<LiveFrameStream>,
    monitor: Option<gdk::Monitor>,
) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Kwybars")
        .build();

    window.set_widget_name("kwybars-overlay");
    style::strip_background_classes(&window);
    window.set_decorated(false);
    window.set_resizable(false);
    window.set_focusable(false);

    let drawing_area = build_drawing_area(config, stream, theme_palette);
    window.set_child(Some(&drawing_area));

    layer::apply_default_size(&window, config, monitor.as_ref());
    layer::configure_layer_shell(&window, config, monitor.as_ref());

    window.present();
    window
}

fn build_drawing_area(
    config: &AppConfig,
    stream: Rc<LiveFrameStream>,
    theme_palette: Option<ThemePalette>,
) -> gtk::DrawingArea {
    let position = config.overlay.position.clone();
    let is_horizontal = matches!(position, OverlayPosition::Bottom | OverlayPosition::Top);
    let is_left = matches!(position, OverlayPosition::Left);
    let is_top = matches!(position, OverlayPosition::Top);
    let is_radial = config.visualizer.layout == VisualizerLayout::Radial;
    let is_frame = config.visualizer.layout == VisualizerLayout::Frame;
    let is_polygon = config.visualizer.layout == VisualizerLayout::Polygon;
    let is_centered = matches!(
        config.visualizer.layout,
        VisualizerLayout::Frame | VisualizerLayout::Radial | VisualizerLayout::Polygon
    );
    let bar_thickness = f64::from(config.visualizer.bar_width.max(1));
    let corner_radius = f64::from(config.visualizer.bar_corner_radius.max(0.0));
    let gap = f64::from(config.visualizer.gap);
    let bar_style = draw::BarStyle {
        thickness: bar_thickness,
        gap,
        corner_radius,
        segmented: config.visualizer.segmented_bars,
        segment_length: f64::from(config.visualizer.segment_length.max(1)),
        segment_gap: f64::from(config.visualizer.segment_gap),
    };
    let bar_count = config.visualizer.bars.max(1);
    let fps = config.visualizer.framerate.max(1);
    let interval_ms = (1000_u64 / u64::from(fps)).max(1);
    let bar_color_mode = config.visualizer.color_mode;
    let bar_color = config.visualizer.color_rgba;
    let bar_color2 = config.visualizer.color2_rgba;
    let radial_inner_radius = f64::from(config.visualizer.radial_inner_radius.max(1));
    let radial_start_angle = f64::from(config.visualizer.radial_start_angle).to_radians();
    let radial_arc_radians = f64::from(config.visualizer.radial_arc_degrees).to_radians();
    let radial_rotation_radians_per_second =
        f64::from(config.visualizer.radial_rotation_speed).to_radians();
    let center_offset_x = f64::from(config.visualizer.center_offset_x);
    let center_offset_y = f64::from(config.visualizer.center_offset_y);
    let polygon_sides = config.visualizer.polygon_sides.max(3) as usize;
    let polygon_radius = f64::from(config.visualizer.polygon_radius.max(1));
    let polygon_rotation = f64::from(config.visualizer.polygon_rotation).to_radians();
    let polygon_rotation_radians_per_second =
        f64::from(config.visualizer.polygon_rotation_speed).to_radians();
    let frame_edges = config.visualizer.frame_edges.clone();
    let frame_mirror_mode = config.visualizer.frame_mirror_mode;
    let frame_top_thickness = f64::from(config.overlay.height.max(1));
    let frame_bottom_thickness = f64::from(config.overlay.height.max(1));
    let frame_left_thickness = f64::from(config.overlay.width.max(1));
    let frame_right_thickness = f64::from(config.overlay.width.max(1));
    let frame_anchor = f64::from(config.overlay.anchor_margin);
    let frame_margin_left = f64::from(config.overlay.margin_left);
    let frame_margin_right = f64::from(config.overlay.margin_right);
    let frame_margin_top = f64::from(config.overlay.margin_top);
    let frame_margin_bottom = f64::from(config.overlay.margin_bottom);
    let theme_colors = theme_palette
        .map(|theme| theme.colors)
        .filter(|colors| !colors.is_empty());

    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_widget_name("kwybars-bars");
    style::strip_background_classes(&drawing_area);
    drawing_area.set_can_target(false);

    if is_centered {
        drawing_area.set_hexpand(true);
        drawing_area.set_vexpand(true);
    } else if is_horizontal {
        drawing_area.set_content_height(to_i32(config.overlay.height));
        if !config.overlay.full_length {
            drawing_area.set_content_width(to_i32(config.overlay.width));
        }
        drawing_area.set_hexpand(config.overlay.full_length);
        drawing_area.set_vexpand(false);
    } else {
        drawing_area.set_content_width(to_i32(config.overlay.width));
        if !config.overlay.full_length {
            drawing_area.set_content_height(to_i32(config.overlay.height));
        }
        drawing_area.set_hexpand(false);
        drawing_area.set_vexpand(config.overlay.full_length);
    }

    let bar_values = Rc::new(RefCell::new(vec![0.0_f64; bar_count]));
    let rotation_started_at = Instant::now();

    {
        let values_for_draw = Rc::clone(&bar_values);
        drawing_area.set_draw_func(move |_, ctx, width, height| {
            let values = values_for_draw.borrow();
            if values.is_empty() || width <= 0 || height <= 0 {
                return;
            }

            if is_radial {
                let center_x = (f64::from(width) * 0.5) + center_offset_x;
                let center_y = (f64::from(height) * 0.5) + center_offset_y;
                let animated_start_angle = radial_start_angle
                    + (rotation_started_at.elapsed().as_secs_f64()
                        * radial_rotation_radians_per_second);

                draw::for_each_radial_bar(
                    &values,
                    draw::RadialLayout {
                        width: f64::from(width),
                        height: f64::from(height),
                        inner_radius: radial_inner_radius,
                        start_angle: animated_start_angle,
                        arc_radians: radial_arc_radians,
                    },
                    bar_style,
                    |index, spec| {
                        let color = if let Some(colors) = theme_colors.as_ref() {
                            let color_idx =
                                draw::bar_color_index(index, values.len(), colors.len());
                            colors[color_idx]
                        } else {
                            color_for_index(
                                bar_color_mode,
                                bar_color,
                                bar_color2,
                                index,
                                values.len(),
                            )
                        };

                        ctx.set_source_rgba(
                            f64::from(color.r),
                            f64::from(color.g),
                            f64::from(color.b),
                            f64::from(color.a),
                        );
                        draw::append_radial_bar_path(ctx, center_x, center_y, spec, bar_style);
                        if ctx.fill().is_err() {
                            error!("kwybars: cairo fill failed");
                        }
                    },
                );
                return;
            }

            if is_frame {
                let active_edges = normalized_frame_edges(&frame_edges);
                if active_edges.is_empty() {
                    return;
                }

                let frame_metrics = FrameMetrics {
                    width: f64::from(width),
                    height: f64::from(height),
                    top_thickness: frame_top_thickness,
                    bottom_thickness: frame_bottom_thickness,
                    left_thickness: frame_left_thickness,
                    right_thickness: frame_right_thickness,
                    anchor_margin: frame_anchor,
                    margin_left: frame_margin_left,
                    margin_right: frame_margin_right,
                    margin_top: frame_margin_top,
                    margin_bottom: frame_margin_bottom,
                };

                for (edge_index, edge) in active_edges.iter().enumerate() {
                    let edge_slice = resolve_frame_edge_slice(
                        &values,
                        &active_edges,
                        edge_index,
                        frame_mirror_mode,
                    );
                    if edge_slice.values.is_empty() {
                        continue;
                    }

                    let edge_rect = frame_edge_rect(edge.clone(), frame_metrics);
                    let edge_paint = EdgePaint {
                        ctx,
                        total_count: values.len(),
                        global_offset: edge_slice.global_offset,
                        style: bar_style,
                        color_mode: bar_color_mode,
                        color: bar_color,
                        color2: bar_color2,
                        theme_colors: theme_colors.as_deref(),
                    };
                    paint_line_edge(edge_slice.values, edge_rect, &edge_paint);
                }
                return;
            }

            if is_polygon {
                let center_x = (f64::from(width) * 0.5) + center_offset_x;
                let center_y = (f64::from(height) * 0.5) + center_offset_y;
                let animated_polygon_rotation = polygon_rotation
                    + (rotation_started_at.elapsed().as_secs_f64()
                        * polygon_rotation_radians_per_second);

                draw::for_each_polygon_bar(
                    &values,
                    draw::PolygonLayout {
                        width: f64::from(width),
                        height: f64::from(height),
                        radius: polygon_radius,
                        rotation_radians: animated_polygon_rotation,
                        sides: polygon_sides,
                    },
                    bar_style,
                    |index, spec| {
                        let color = if let Some(colors) = theme_colors.as_ref() {
                            let color_idx =
                                draw::bar_color_index(index, values.len(), colors.len());
                            colors[color_idx]
                        } else {
                            color_for_index(
                                bar_color_mode,
                                bar_color,
                                bar_color2,
                                index,
                                values.len(),
                            )
                        };

                        ctx.set_source_rgba(
                            f64::from(color.r),
                            f64::from(color.g),
                            f64::from(color.b),
                            f64::from(color.a),
                        );
                        draw::append_directed_bar_path(ctx, center_x, center_y, spec, bar_style);
                        if ctx.fill().is_err() {
                            error!("kwybars: cairo fill failed");
                        }
                    },
                );
                return;
            }

            if let Some(colors) = theme_colors.as_ref() {
                if is_horizontal {
                    draw::for_each_horizontal_bar(
                        &values,
                        f64::from(width),
                        f64::from(height),
                        bar_style.thickness,
                        bar_style.gap,
                        is_top,
                        |index, x, y, bar_width, bar_height| {
                            let color_idx =
                                draw::bar_color_index(index, values.len(), colors.len());
                            let color = colors[color_idx];
                            ctx.set_source_rgba(
                                f64::from(color.r),
                                f64::from(color.g),
                                f64::from(color.b),
                                f64::from(color.a),
                            );
                            draw::append_bar_path(
                                ctx,
                                draw::BarRect {
                                    x,
                                    y,
                                    width: bar_width,
                                    height: bar_height,
                                },
                                bar_style,
                                draw::BarOrientation::Horizontal,
                                is_top,
                            );
                            if ctx.fill().is_err() {
                                error!("kwybars: cairo fill failed");
                            }
                        },
                    );
                } else {
                    draw::for_each_vertical_bar(
                        &values,
                        f64::from(width),
                        f64::from(height),
                        bar_style.thickness,
                        bar_style.gap,
                        is_left,
                        |index, x, y, bar_width, bar_height| {
                            let color_idx =
                                draw::bar_color_index(index, values.len(), colors.len());
                            let color = colors[color_idx];
                            ctx.set_source_rgba(
                                f64::from(color.r),
                                f64::from(color.g),
                                f64::from(color.b),
                                f64::from(color.a),
                            );
                            draw::append_bar_path(
                                ctx,
                                draw::BarRect {
                                    x,
                                    y,
                                    width: bar_width,
                                    height: bar_height,
                                },
                                bar_style,
                                draw::BarOrientation::Vertical,
                                is_left,
                            );
                            if ctx.fill().is_err() {
                                error!("kwybars: cairo fill failed");
                            }
                        },
                    );
                }
                return;
            }

            match bar_color_mode {
                VisualizerColorMode::Solid => {
                    ctx.set_source_rgba(
                        f64::from(bar_color.r),
                        f64::from(bar_color.g),
                        f64::from(bar_color.b),
                        f64::from(bar_color.a),
                    );
                }
                VisualizerColorMode::Gradient => {
                    let (x0, y0, x1, y1) = if is_horizontal {
                        if is_top {
                            (0.0, 0.0, 0.0, f64::from(height))
                        } else {
                            (0.0, f64::from(height), 0.0, 0.0)
                        }
                    } else if is_left {
                        (0.0, 0.0, f64::from(width), 0.0)
                    } else {
                        (f64::from(width), 0.0, 0.0, 0.0)
                    };

                    let gradient = gtk::cairo::LinearGradient::new(x0, y0, x1, y1);
                    gradient.add_color_stop_rgba(
                        0.0,
                        f64::from(bar_color.r),
                        f64::from(bar_color.g),
                        f64::from(bar_color.b),
                        f64::from(bar_color.a),
                    );
                    gradient.add_color_stop_rgba(
                        1.0,
                        f64::from(bar_color2.r),
                        f64::from(bar_color2.g),
                        f64::from(bar_color2.b),
                        f64::from(bar_color2.a),
                    );

                    if ctx.set_source(&gradient).is_err() {
                        ctx.set_source_rgba(
                            f64::from(bar_color.r),
                            f64::from(bar_color.g),
                            f64::from(bar_color.b),
                            f64::from(bar_color.a),
                        );
                    }
                }
            }

            if is_horizontal {
                draw::draw_horizontal_bars(
                    ctx,
                    &values,
                    f64::from(width),
                    f64::from(height),
                    bar_style,
                    is_top,
                );
            } else {
                draw::draw_vertical_bars(
                    ctx,
                    &values,
                    f64::from(width),
                    f64::from(height),
                    bar_style,
                    is_left,
                );
            }

            if ctx.fill().is_err() {
                error!("kwybars: cairo fill failed");
            }
        });
    }

    {
        let stream_for_tick = Rc::clone(&stream);
        let values_for_tick = Rc::clone(&bar_values);
        let drawing_area_weak = drawing_area.downgrade();
        glib::timeout_add_local(Duration::from_millis(interval_ms), move || {
            let Some(drawing_area_for_tick) = drawing_area_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };

            let frame = stream_for_tick.latest_frame();
            let mut target = values_for_tick.borrow_mut();
            if target.len() != frame.bars.len() {
                target.resize(frame.bars.len(), 0.0);
            }

            for (slot, value) in target.iter_mut().zip(frame.bars.iter()) {
                *slot = f64::from(*value);
            }

            drawing_area_for_tick.queue_draw();
            glib::ControlFlow::Continue
        });
    }

    drawing_area
}

fn normalized_frame_edges(edges: &[OverlayPosition]) -> Vec<OverlayPosition> {
    let mut normalized = Vec::new();
    for edge in edges {
        if !normalized.contains(edge) {
            normalized.push(edge.clone());
        }
    }
    normalized
}

fn resolve_frame_edge_slice<'a>(
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

fn frame_edge_rect(edge: OverlayPosition, metrics: FrameMetrics) -> draw::FrameEdgeRect {
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

fn paint_line_edge(values: &[f64], edge_rect: draw::FrameEdgeRect, edge_paint: &EdgePaint<'_>) {
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

fn to_i32(value: u32) -> i32 {
    value.max(1).min(i32::MAX as u32) as i32
}

fn color_for_index(
    mode: VisualizerColorMode,
    start: RgbaColor,
    end: RgbaColor,
    index: usize,
    count: usize,
) -> RgbaColor {
    if mode == VisualizerColorMode::Solid || count <= 1 {
        return start;
    }

    let t = index as f32 / (count.saturating_sub(1)) as f32;
    RgbaColor {
        r: lerp(start.r, end.r, t),
        g: lerp(start.g, end.g, t),
        b: lerp(start.b, end.b, t),
        a: lerp(start.a, end.a, t),
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + ((end - start) * t.clamp(0.0, 1.0))
}
