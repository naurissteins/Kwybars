use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;

use gtk::glib;
use gtk::prelude::*;
use kwybars_common::config::{
    AppConfig, LineMode, MirrorOrientation, OverlayPosition, RgbaColor, VisualizerColorMode,
    VisualizerLayout,
};
use kwybars_common::theme::ThemePalette;
use kwybars_engine::live::LiveFrameStream;
use tracing::error;

use super::color::color_for_index;
use super::draw;
use super::frame::{
    EdgePaint, FrameMetrics, frame_edge_rect, normalized_frame_edges, paint_line_edge,
    resolve_frame_edge_slice,
};
use super::style;

#[derive(Clone, Copy, Default)]
struct FloatingParticle {
    offset: f64,
    velocity: f64,
}

#[derive(Clone, Copy)]
struct WaveSource<'a> {
    axis_start: (f64, f64),
    axis_end: (f64, f64),
    color_mode: VisualizerColorMode,
    color: RgbaColor,
    color2: RgbaColor,
    theme_colors: Option<&'a [RgbaColor]>,
    alpha_scale: f64,
}

pub(super) fn build_drawing_area(
    config: &AppConfig,
    stream: Rc<LiveFrameStream>,
    theme_palette: Option<ThemePalette>,
) -> gtk::DrawingArea {
    let position = config.overlay.position.clone();
    let is_horizontal = matches!(position, OverlayPosition::Bottom | OverlayPosition::Top);
    let is_left = matches!(position, OverlayPosition::Left);
    let is_top = matches!(position, OverlayPosition::Top);
    let is_radial = config.visualizer.layout == VisualizerLayout::Radial;
    let is_mirror = config.visualizer.layout == VisualizerLayout::Mirror;
    let is_wave = config.visualizer.layout == VisualizerLayout::Wave;
    let is_frame = config.visualizer.layout == VisualizerLayout::Frame;
    let is_polygon = config.visualizer.layout == VisualizerLayout::Polygon;
    let is_particle = config.visualizer.layout == VisualizerLayout::Particle;
    let is_floating = config.visualizer.layout == VisualizerLayout::Floating;
    let is_centered = matches!(
        config.visualizer.layout,
        VisualizerLayout::Mirror
            | VisualizerLayout::Frame
            | VisualizerLayout::Radial
            | VisualizerLayout::Polygon
    );
    let mirror_horizontal = matches!(
        config.visualizer.mirror_orientation,
        MirrorOrientation::Horizontal
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
    let line_mode = match config.visualizer.line_mode {
        LineMode::Continuous => draw::LinearBarMode::Continuous,
        LineMode::Split => draw::LinearBarMode::Split {
            center_gap: f64::from(config.visualizer.line_split_gap),
        },
    };
    let mirror_gap = f64::from(config.visualizer.mirror_gap);
    let wave_stroke_width = f64::from(config.visualizer.wave_stroke_width.max(1));
    let wave_fill = config.visualizer.wave_fill;
    let wave_glow = config.visualizer.wave_glow;
    let wave_smoothing = f64::from(config.visualizer.wave_smoothing);
    let wave_motion_smoothing = f64::from(config.visualizer.wave_motion_smoothing.clamp(0.0, 1.0));
    let wave_amplitude = f64::from(config.visualizer.wave_amplitude);
    let radial_inner_radius = f64::from(config.visualizer.radial_inner_radius.max(1));
    let radial_start_angle = f64::from(config.visualizer.radial_start_angle).to_radians();
    let radial_arc_radians = f64::from(config.visualizer.radial_arc_degrees).to_radians();
    let radial_rotation_radians_per_second =
        f64::from(config.visualizer.radial_rotation_speed).to_radians();
    let center_offset_x = f64::from(config.visualizer.center_offset_x);
    let center_offset_y = f64::from(config.visualizer.center_offset_y);
    let polygon_sides = config.visualizer.polygon_sides.max(3) as usize;
    let polygon_radius = f64::from(config.visualizer.polygon_radius.max(1));
    let polygon_bar_length = f64::from(config.visualizer.polygon_bar_length);
    let polygon_rotation = f64::from(config.visualizer.polygon_rotation).to_radians();
    let polygon_rotation_radians_per_second =
        f64::from(config.visualizer.polygon_rotation_speed).to_radians();
    let frame_edges = config.visualizer.frame_edges.clone();
    let frame_mirror_mode = config.visualizer.frame_mirror_mode;
    let overlay_full_length = config.overlay.full_length;
    let overlay_width = f64::from(config.overlay.width.max(1));
    let overlay_height = f64::from(config.overlay.height.max(1));
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
        if !overlay_full_length {
            drawing_area.set_content_width(to_i32(config.overlay.width));
        }
        drawing_area.set_hexpand(overlay_full_length);
        drawing_area.set_vexpand(false);
    } else {
        drawing_area.set_content_width(to_i32(config.overlay.width));
        if !overlay_full_length {
            drawing_area.set_content_height(to_i32(config.overlay.height));
        }
        drawing_area.set_hexpand(false);
        drawing_area.set_vexpand(overlay_full_length);
    }

    let bar_values = Rc::new(RefCell::new(vec![0.0_f64; bar_count]));
    let particle_state = Rc::new(RefCell::new(vec![FloatingParticle::default(); bar_count]));
    let particle_offsets = Rc::new(RefCell::new(vec![0.0_f64; bar_count]));
    let rotation_started_at = Instant::now();

    {
        let values_for_draw = Rc::clone(&bar_values);
        let offsets_for_draw = Rc::clone(&particle_offsets);
        drawing_area.set_draw_func(move |_, ctx, width, height| {
            let values = values_for_draw.borrow();
            if values.is_empty() || width <= 0 || height <= 0 {
                return;
            }

            if is_floating {
                let floating_orientation = if is_horizontal {
                    draw::BarOrientation::Horizontal
                } else {
                    draw::BarOrientation::Vertical
                };
                let from_start = is_top || is_left;

                draw::for_each_floating_particle(
                    &values,
                    &offsets_for_draw.borrow(),
                    draw::FloatingParticleLayout {
                        width: f64::from(width),
                        height: f64::from(height),
                        max_radius: bar_thickness,
                        gap,
                        orientation: floating_orientation,
                        from_start,
                    },
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
                        draw::draw_particle(ctx, spec);
                        if ctx.fill().is_err() {
                            error!("kwybars: cairo fill failed");
                        }
                    },
                );
                return;
            }

            if is_particle {
                let particle_orientation = if is_horizontal {
                    draw::BarOrientation::Horizontal
                } else {
                    draw::BarOrientation::Vertical
                };

                draw::for_each_particle(
                    &values,
                    f64::from(width),
                    f64::from(height),
                    bar_thickness,
                    gap,
                    particle_orientation,
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
                        draw::draw_particle(ctx, spec);
                        if ctx.fill().is_err() {
                            error!("kwybars: cairo fill failed");
                        }
                    },
                );
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

            if is_wave {
                let glow_stroke_width = (wave_stroke_width * 3.0).max(wave_stroke_width + 2.0);
                let wave_padding_extra = 6.0;
                let wave_edge_padding = if wave_glow {
                    (glow_stroke_width * 0.5) + wave_padding_extra
                } else {
                    (wave_stroke_width * 0.5) + wave_padding_extra
                };
                let axis_start = (0.0, 0.0);
                let axis_end = if is_horizontal {
                    (f64::from(width), 0.0)
                } else {
                    (0.0, f64::from(height))
                };
                let wave_source = WaveSource {
                    axis_start,
                    axis_end,
                    color_mode: bar_color_mode,
                    color: bar_color,
                    color2: bar_color2,
                    theme_colors: theme_colors.as_deref(),
                    alpha_scale: 1.0,
                };
                let wave_layout = draw::WaveLayout {
                    width: f64::from(width),
                    height: f64::from(height),
                    stroke_width: wave_stroke_width,
                    edge_padding: wave_edge_padding,
                    smoothing: wave_smoothing,
                    amplitude: wave_amplitude,
                    from_start: is_top || is_left,
                    mode: line_mode,
                };

                ctx.set_line_width(wave_stroke_width);
                ctx.set_line_cap(gtk::cairo::LineCap::Round);
                ctx.set_line_join(gtk::cairo::LineJoin::Round);

                if wave_glow {
                    let glow_source = WaveSource {
                        alpha_scale: 0.18,
                        ..wave_source
                    };
                    set_wave_source(ctx, glow_source);
                    ctx.set_line_width(glow_stroke_width);
                    if is_horizontal {
                        draw::append_horizontal_wave_path(ctx, &values, wave_layout, 0.0, 0.0);
                    } else {
                        draw::append_vertical_wave_path(ctx, &values, wave_layout, 0.0, 0.0);
                    }
                    if ctx.stroke().is_err() {
                        error!("kwybars: cairo stroke failed");
                    }
                    ctx.set_line_width(wave_stroke_width);
                }

                if wave_fill {
                    let fill_source = WaveSource {
                        alpha_scale: 0.24,
                        ..wave_source
                    };
                    set_wave_source(ctx, fill_source);
                    if is_horizontal {
                        draw::append_horizontal_wave_fill_path(ctx, &values, wave_layout, 0.0, 0.0);
                    } else {
                        draw::append_vertical_wave_fill_path(ctx, &values, wave_layout, 0.0, 0.0);
                    }
                    if ctx.fill().is_err() {
                        error!("kwybars: cairo fill failed");
                    }
                }

                set_wave_source(ctx, wave_source);

                if is_horizontal {
                    draw::append_horizontal_wave_path(ctx, &values, wave_layout, 0.0, 0.0);
                } else {
                    draw::append_vertical_wave_path(ctx, &values, wave_layout, 0.0, 0.0);
                }

                if ctx.stroke().is_err() {
                    error!("kwybars: cairo stroke failed");
                }
                return;
            }

            if is_mirror {
                let total_width = f64::from(width);
                let total_height = f64::from(height);
                let (active_x, active_y, active_width, active_height) = if mirror_horizontal {
                    let active_width = if overlay_full_length {
                        total_width.max(1.0)
                    } else {
                        overlay_width.min(total_width.max(1.0))
                    };
                    let active_height = overlay_height.min(total_height.max(1.0));
                    let active_x = if overlay_full_length {
                        center_offset_x
                    } else {
                        ((total_width - active_width) * 0.5) + center_offset_x
                    };
                    let active_y = ((total_height - active_height) * 0.5) + center_offset_y;
                    (active_x, active_y, active_width, active_height)
                } else {
                    let active_width = overlay_width.min(total_width.max(1.0));
                    let active_height = if overlay_full_length {
                        total_height.max(1.0)
                    } else {
                        overlay_height.min(total_height.max(1.0))
                    };
                    let active_x = ((total_width - active_width) * 0.5) + center_offset_x;
                    let active_y = if overlay_full_length {
                        center_offset_y
                    } else {
                        ((total_height - active_height) * 0.5) + center_offset_y
                    };
                    (active_x, active_y, active_width, active_height)
                };

                if let Some(colors) = theme_colors.as_ref() {
                    if mirror_horizontal {
                        draw::for_each_horizontal_mirror_bar_mode(
                            &values,
                            draw::MirrorHorizontalLayout {
                                width: active_width,
                                height: active_height,
                                bar_thickness: bar_style.thickness,
                                gap: bar_style.gap,
                                mirror_gap,
                                mode: line_mode,
                            },
                            |index, x, bar_width, half_height, half_gap| {
                                let color_idx =
                                    draw::bar_color_index(index, values.len(), colors.len());
                                let color = colors[color_idx];
                                let center_y = active_y + (active_height * 0.5);
                                ctx.set_source_rgba(
                                    f64::from(color.r),
                                    f64::from(color.g),
                                    f64::from(color.b),
                                    f64::from(color.a),
                                );
                                draw::append_bar_path(
                                    ctx,
                                    draw::BarRect {
                                        x: active_x + x,
                                        y: center_y - half_gap - half_height,
                                        width: bar_width,
                                        height: half_height,
                                    },
                                    bar_style,
                                    draw::BarOrientation::Horizontal,
                                    false,
                                );
                                draw::append_bar_path(
                                    ctx,
                                    draw::BarRect {
                                        x: active_x + x,
                                        y: center_y + half_gap,
                                        width: bar_width,
                                        height: half_height,
                                    },
                                    bar_style,
                                    draw::BarOrientation::Horizontal,
                                    true,
                                );
                                if ctx.fill().is_err() {
                                    error!("kwybars: cairo fill failed");
                                }
                            },
                        );
                    } else {
                        draw::for_each_vertical_mirror_bar_mode(
                            &values,
                            draw::MirrorVerticalLayout {
                                width: active_width,
                                height: active_height,
                                bar_thickness: bar_style.thickness,
                                gap: bar_style.gap,
                                mirror_gap,
                                mode: line_mode,
                            },
                            |index, y, bar_height, half_width, half_gap| {
                                let color_idx =
                                    draw::bar_color_index(index, values.len(), colors.len());
                                let color = colors[color_idx];
                                let center_x = active_x + (active_width * 0.5);
                                ctx.set_source_rgba(
                                    f64::from(color.r),
                                    f64::from(color.g),
                                    f64::from(color.b),
                                    f64::from(color.a),
                                );
                                draw::append_bar_path(
                                    ctx,
                                    draw::BarRect {
                                        x: center_x - half_gap - half_width,
                                        y: active_y + y,
                                        width: half_width,
                                        height: bar_height,
                                    },
                                    bar_style,
                                    draw::BarOrientation::Vertical,
                                    false,
                                );
                                draw::append_bar_path(
                                    ctx,
                                    draw::BarRect {
                                        x: center_x + half_gap,
                                        y: active_y + y,
                                        width: half_width,
                                        height: bar_height,
                                    },
                                    bar_style,
                                    draw::BarOrientation::Vertical,
                                    true,
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
                        let (x0, y0, x1, y1) = if mirror_horizontal {
                            (active_x, active_y, active_x, active_y + active_height)
                        } else {
                            (active_x, active_y, active_x + active_width, active_y)
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

                if mirror_horizontal {
                    draw::draw_horizontal_mirror_bars_mode(
                        ctx,
                        &values,
                        draw::MirrorHorizontalLayout {
                            width: active_width,
                            height: active_height,
                            bar_thickness: bar_style.thickness,
                            gap: bar_style.gap,
                            mirror_gap,
                            mode: line_mode,
                        },
                        bar_style,
                        active_x,
                        active_y,
                    );
                } else {
                    draw::draw_vertical_mirror_bars_mode(
                        ctx,
                        &values,
                        draw::MirrorVerticalLayout {
                            width: active_width,
                            height: active_height,
                            bar_thickness: bar_style.thickness,
                            gap: bar_style.gap,
                            mirror_gap,
                            mode: line_mode,
                        },
                        bar_style,
                        active_x,
                        active_y,
                    );
                }

                if ctx.fill().is_err() {
                    error!("kwybars: cairo fill failed");
                }
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
                        bar_length: polygon_bar_length,
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
                    draw::for_each_horizontal_bar_mode(
                        &values,
                        draw::HorizontalBarLayout {
                            width: f64::from(width),
                            height: f64::from(height),
                            bar_thickness: bar_style.thickness,
                            gap: bar_style.gap,
                            from_top: is_top,
                            mode: line_mode,
                        },
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
                    draw::for_each_vertical_bar_mode(
                        &values,
                        draw::VerticalBarLayout {
                            width: f64::from(width),
                            height: f64::from(height),
                            bar_thickness: bar_style.thickness,
                            gap: bar_style.gap,
                            from_left: is_left,
                            mode: line_mode,
                        },
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
                draw::draw_horizontal_bars_mode(
                    ctx,
                    &values,
                    draw::HorizontalBarLayout {
                        width: f64::from(width),
                        height: f64::from(height),
                        bar_thickness: bar_style.thickness,
                        gap: bar_style.gap,
                        from_top: is_top,
                        mode: line_mode,
                    },
                    bar_style,
                );
            } else {
                draw::draw_vertical_bars_mode(
                    ctx,
                    &values,
                    draw::VerticalBarLayout {
                        width: f64::from(width),
                        height: f64::from(height),
                        bar_thickness: bar_style.thickness,
                        gap: bar_style.gap,
                        from_left: is_left,
                        mode: line_mode,
                    },
                    bar_style,
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
        let physics_for_tick = Rc::clone(&particle_state);
        let offsets_for_tick = Rc::clone(&particle_offsets);
        let drawing_area_weak = drawing_area.downgrade();

        let gravity = 0.042_f64;
        let jump_factor = 0.10_f64;

        glib::timeout_add_local(Duration::from_millis(interval_ms), move || {
            let Some(drawing_area_for_tick) = drawing_area_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };

            let frame = stream_for_tick.latest_frame();
            let mut target = values_for_tick.borrow_mut();
            let mut physics = physics_for_tick.borrow_mut();
            let mut offsets = offsets_for_tick.borrow_mut();

            if target.len() != frame.bars.len() {
                target.resize(frame.bars.len(), 0.0);
                physics.resize(frame.bars.len(), FloatingParticle::default());
                offsets.resize(frame.bars.len(), 0.0);
            }

            for (((slot, value), p), off) in target
                .iter_mut()
                .zip(frame.bars.iter())
                .zip(physics.iter_mut())
                .zip(offsets.iter_mut())
            {
                let val = f64::from(*value);
                if is_wave {
                    let response = if val > *slot {
                        (wave_motion_smoothing * 1.35).min(1.0)
                    } else {
                        (wave_motion_smoothing * 0.55).min(1.0)
                    };
                    *slot += (val - *slot) * response;
                } else {
                    *slot = val;
                }

                // Physics update: energy gives an upward impulse
                if val > 0.05 {
                    p.velocity += val * jump_factor;
                }
                p.velocity -= gravity;
                p.offset += p.velocity;

                if p.offset < 0.0 {
                    p.offset = 0.0;
                    p.velocity = 0.0;
                } else if p.offset > 1.0 {
                    p.offset = 1.0;
                    p.velocity = -p.velocity * 0.2; // Small bounce off the ceiling
                }
                *off = p.offset;
            }

            drawing_area_for_tick.queue_draw();
            glib::ControlFlow::Continue
        });
    }

    drawing_area
}

fn to_i32(value: u32) -> i32 {
    value.max(1).min(i32::MAX as u32) as i32
}

fn set_wave_source(ctx: &gtk::cairo::Context, source: WaveSource<'_>) {
    if let Some(colors) = source.theme_colors {
        if colors.len() == 1 {
            let (r, g, b, a) = scaled_rgba(colors[0], source.alpha_scale);
            ctx.set_source_rgba(r, g, b, a);
            return;
        }

        let gradient = gtk::cairo::LinearGradient::new(
            source.axis_start.0,
            source.axis_start.1,
            source.axis_end.0,
            source.axis_end.1,
        );
        let stop_denom = (colors.len().saturating_sub(1)).max(1) as f64;
        for (index, color) in colors.iter().enumerate() {
            let (r, g, b, a) = scaled_rgba(*color, source.alpha_scale);
            gradient.add_color_stop_rgba(index as f64 / stop_denom, r, g, b, a);
        }
        if ctx.set_source(&gradient).is_ok() {
            return;
        }

        let (r, g, b, a) = scaled_rgba(colors[0], source.alpha_scale);
        ctx.set_source_rgba(r, g, b, a);
        return;
    }

    match source.color_mode {
        VisualizerColorMode::Solid => {
            let (r, g, b, a) = scaled_rgba(source.color, source.alpha_scale);
            ctx.set_source_rgba(r, g, b, a);
        }
        VisualizerColorMode::Gradient => {
            let gradient = gtk::cairo::LinearGradient::new(
                source.axis_start.0,
                source.axis_start.1,
                source.axis_end.0,
                source.axis_end.1,
            );
            let (r1, g1, b1, a1) = scaled_rgba(source.color, source.alpha_scale);
            let (r2, g2, b2, a2) = scaled_rgba(source.color2, source.alpha_scale);
            gradient.add_color_stop_rgba(0.0, r1, g1, b1, a1);
            gradient.add_color_stop_rgba(1.0, r2, g2, b2, a2);
            if ctx.set_source(&gradient).is_err() {
                ctx.set_source_rgba(r1, g1, b1, a1);
            }
        }
    }
}

fn scaled_rgba(color: RgbaColor, alpha_scale: f64) -> (f64, f64, f64, f64) {
    (
        f64::from(color.r),
        f64::from(color.g),
        f64::from(color.b),
        (f64::from(color.a) * alpha_scale).clamp(0.0, 1.0),
    )
}
