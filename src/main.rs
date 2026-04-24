mod config;
mod lsp;
mod ui;
mod utils;

use adw::{Application, prelude::*};
use gtk::{CssProvider, gdk::Display};

use crate::ui::window::EditorWindow;

fn main() {
    let application = adw::Application::new(Some("ca.potatoe.spikedit"), Default::default());
    application.connect_startup(load_css);
    application.connect_activate(build_ui);
    // Set up accels
    application.set_accels_for_action("win.tab.close", &["<Ctrl>W"]);
    application.set_accels_for_action("win.tab.new_scratch", &["<Ctrl>T"]);
    application.set_accels_for_action("win.new_window", &["<Ctrl>N"]);
    application.set_accels_for_action("win.open.file", &["<Ctrl>O"]);
    application.set_accels_for_action("win.open.folder", &["<Ctrl><Shift>O"]);
    application.set_accels_for_action("win.save", &["<Ctrl>S"]);
    application.set_accels_for_action("win.save_as", &["<Ctrl><Shift>S"]);
    application.set_accels_for_action("win.search.toggle", &["<Ctrl>F"]);
    // run
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
