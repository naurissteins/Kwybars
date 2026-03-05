mod draw;
mod layer;
mod style;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use kwybars_common::config::{AppConfig, OverlayPosition, VisualizerColorMode};
use kwybars_engine::live::LiveFrameStream;

pub fn build_overlay_window(app: &gtk::Application, config: AppConfig) -> gtk::ApplicationWindow {
    style::install_css();

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Kwybars")
        .build();

    window.set_widget_name("kwybars-overlay");
    window.set_decorated(false);
    window.set_resizable(false);
    window.set_focusable(false);

    let drawing_area = build_drawing_area(&config);
    window.set_child(Some(&drawing_area));

    layer::apply_default_size(&window, &config.overlay);
    layer::configure_layer_shell(&window, &config.overlay);

    window.present();

    window
}

fn build_drawing_area(config: &AppConfig) -> gtk::DrawingArea {
    let position = config.overlay.position.clone();
    let is_horizontal = matches!(position, OverlayPosition::Bottom | OverlayPosition::Top);
    let is_left = matches!(position, OverlayPosition::Left);
    let is_top = matches!(position, OverlayPosition::Top);
    let bar_thickness = f64::from(config.visualizer.bar_width.max(1));
    let gap = f64::from(config.visualizer.gap);
    let bar_count = config.visualizer.bars.max(1);
    let fps = config.visualizer.framerate.max(1);
    let interval_ms = (1000_u64 / u64::from(fps)).max(1);
    let bar_color_mode = config.visualizer.color_mode;
    let bar_color = config.visualizer.color_rgba;
    let bar_color2 = config.visualizer.color2_rgba;

    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_widget_name("kwybars-bars");
    drawing_area.set_can_target(false);

    if is_horizontal {
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

    let stream = Rc::new(LiveFrameStream::spawn(config.visualizer.clone()));
    eprintln!("kwybars: using {:?} frame source", stream.source_kind());
    let bar_values = Rc::new(RefCell::new(vec![0.0_f64; bar_count]));

    {
        let values_for_draw = Rc::clone(&bar_values);
        drawing_area.set_draw_func(move |_, ctx, width, height| {
            let values = values_for_draw.borrow();
            if values.is_empty() || width <= 0 || height <= 0 {
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
                    bar_thickness,
                    gap,
                    is_top,
                );
            } else {
                draw::draw_vertical_bars(
                    ctx,
                    &values,
                    f64::from(width),
                    f64::from(height),
                    bar_thickness,
                    gap,
                    is_left,
                );
            }

            if ctx.fill().is_err() {
                eprintln!("kwybars: cairo fill failed");
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
