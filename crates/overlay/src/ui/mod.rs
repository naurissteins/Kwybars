mod draw;
mod layer;
mod style;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use kwybars_common::config::{AppConfig, OverlayPosition};
use kwybars_engine::pipeline::{DummySineSource, FrameSource};

const HORIZONTAL_THICKNESS: i32 = 120;
const VERTICAL_THICKNESS: i32 = 150;

pub fn build_overlay_window(app: &gtk::Application, config: AppConfig) {
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

    layer::apply_default_size(&window, &config.overlay.position);
    layer::configure_layer_shell(&window, &config.overlay);

    window.present();
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

    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_widget_name("kwybars-bars");
    drawing_area.set_hexpand(true);
    drawing_area.set_vexpand(true);
    drawing_area.set_can_target(false);

    if is_horizontal {
        drawing_area.set_content_height(HORIZONTAL_THICKNESS);
    } else {
        drawing_area.set_content_width(VERTICAL_THICKNESS);
    }

    let source = Rc::new(RefCell::new(DummySineSource::new(bar_count)));
    let bar_values = Rc::new(RefCell::new(vec![0.0_f64; bar_count]));

    {
        let values_for_draw = Rc::clone(&bar_values);
        drawing_area.set_draw_func(move |_, ctx, width, height| {
            let values = values_for_draw.borrow();
            if values.is_empty() || width <= 0 || height <= 0 {
                return;
            }

            ctx.set_source_rgba(0.12, 0.88, 0.68, 0.9);

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
        let source_for_tick = Rc::clone(&source);
        let values_for_tick = Rc::clone(&bar_values);
        let drawing_area_for_tick = drawing_area.clone();
        glib::timeout_add_local(Duration::from_millis(interval_ms), move || {
            let frame = source_for_tick.borrow_mut().next_frame();
            let mut target = values_for_tick.borrow_mut();

            for (idx, value) in frame.bars.into_iter().enumerate() {
                if let Some(slot) = target.get_mut(idx) {
                    *slot = f64::from(value);
                }
            }

            drawing_area_for_tick.queue_draw();
            glib::ControlFlow::Continue
        });
    }

    drawing_area
}
