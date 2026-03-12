use gtk::gdk;
use gtk::prelude::*;

pub fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "
        window#kwybars-overlay,
        window#kwybars-overlay.background,
        window#kwybars-overlay:backdrop,
        window#kwybars-overlay > *,
        window#kwybars-overlay > *.background,
        drawingarea#kwybars-bars,
        drawingarea#kwybars-bars.background {
            background-color: transparent;
            background-image: none;
            box-shadow: none;
            border: none;
        }
        ",
    );

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );
    }
}

pub fn strip_background_classes(widget: &impl IsA<gtk::Widget>) {
    widget.remove_css_class("background");
}
