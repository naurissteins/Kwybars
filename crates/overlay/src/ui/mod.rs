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
    AppConfig, OverlayPosition, RgbaColor, VisualizerColorMode, VisualizerLayout,
};
use kwybars_engine::live::LiveFrameStream;
use tracing::{error, info};

use crate::theme::ThemePalette;

pub fn build_overlay_windows(
    app: &gtk::Application,
    config: AppConfig,
    theme_palette: Option<ThemePalette>,
) -> Vec<gtk::ApplicationWindow> {
    style::install_css();

    let stream = Rc::new(LiveFrameStream::spawn(config.visualizer.clone()));
    info!("kwybars: using {:?} frame source", stream.source_kind());

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
    let theme_colors = theme_palette
        .map(|theme| theme.colors)
        .filter(|colors| !colors.is_empty());

    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_widget_name("kwybars-bars");
    drawing_area.set_can_target(false);

    if is_radial {
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
                let center_x = f64::from(width) * 0.5;
                let center_y = f64::from(height) * 0.5;
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
