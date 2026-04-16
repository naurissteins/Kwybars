use std::rc::Rc;

use gtk::gdk;
use gtk::prelude::*;
use kwybars_common::config::AppConfig;
use kwybars_common::theme::ThemePalette;
use kwybars_engine::live::LiveFrameStream;
use tracing::info;

use super::layer;
use super::render::build_drawing_area;
use super::style;
use crate::ui::ImageOverlayLayer;

pub fn spawn_frame_stream(config: &AppConfig) -> Rc<LiveFrameStream> {
    let stream = Rc::new(LiveFrameStream::spawn(config.visualizer.clone()));
    info!("kwybars: using {:?} frame source", stream.source_kind());
    stream
}

pub fn build_overlay_windows(
    app: &gtk::Application,
    config: AppConfig,
    theme_palette: Option<ThemePalette>,
    image_overlay: Option<ImageOverlayLayer>,
    stream: Rc<LiveFrameStream>,
) -> Vec<gtk::ApplicationWindow> {
    style::install_css();

    let monitors = layer::selected_monitors(&config.overlay);
    if monitors.is_empty() {
        return vec![build_overlay_window(
            app,
            &config,
            theme_palette.clone(),
            image_overlay.clone(),
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
                image_overlay.clone(),
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
    image_overlay: Option<ImageOverlayLayer>,
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

    let drawing_area = build_drawing_area(config, stream, theme_palette, image_overlay);
    window.set_child(Some(&drawing_area));

    layer::apply_default_size(&window, config, monitor.as_ref());
    layer::configure_layer_shell(&window, config, monitor.as_ref());

    window.present();
    window
}
