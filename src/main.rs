mod ui;

use adw::{Application, prelude::*};
use gtk::{CssProvider, gdk::Display};

use crate::ui::window::EditorWindow;

fn main() {
    let application = adw::Application::new(Some("ca.potatoe.spikedit"), Default::default());
    application.connect_startup(load_css);
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(app: &Application) {
    let window = EditorWindow::new(app);
    window.present();
}

fn load_css(_: &Application) {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("../resources/styles/style.css"));
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
