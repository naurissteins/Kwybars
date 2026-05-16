use std::rc::Rc;

use gtk::prelude::*;
use kwybars_common::config::AppConfig;
use kwybars_common::theme::ThemePalette;
use kwybars_engine::live::LiveFrameStream;
use tracing::info;

use super::render::build_drawing_area;
use super::style;
use super::{layer, output};
use crate::ui::ImageOverlayLayer;

pub fn spawn_frame_stream(config: &AppConfig) -> Rc<LiveFrameStream> {
    let stream = Rc::new(LiveFrameStream::spawn_or_subscribe(
        config.visualizer.clone(),
    ));
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

    output::window_targets(&config)
        .into_iter()
        .map(|target| {
            let target_theme_palette = if target.use_global_theme {
                theme_palette.clone()
            } else {
                None
            };
            build_overlay_window(
                app,
                &target.config,
                target_theme_palette,
                image_overlay.clone(),
                Rc::clone(&stream),
                target.monitor,
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
    monitor: Option<gtk::gdk::Monitor>,
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
