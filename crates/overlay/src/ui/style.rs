use gtk::gdk;

pub fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "
        window#kwybars-overlay,
        window#kwybars-overlay > * {
            background-color: transparent;
            box-shadow: none;
        }

        drawingarea#kwybars-bars {
            background-color: transparent;
        }
        ",
    );

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
