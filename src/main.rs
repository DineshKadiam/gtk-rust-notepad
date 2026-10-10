mod config;
mod lsp;
mod notifications;
mod ui;
mod utils;

use adw::{Application, prelude::*, subclass::prelude::ObjectSubclassIsExt};
use gtk::{
    CssProvider,
    gdk::Display,
    gio::{ApplicationFlags, Cancellable, FileQueryInfoFlags},
};

use crate::ui::window::EditorWindow;

fn main() {
    // Set id to "*.Devel" if ran with debug profile
    #[cfg(not(debug_assertions))]
    let id = Some("ca.potatoe.Kyanite");
    #[cfg(debug_assertions)]
    let id = Some("ca.potatoe.Kyanite.Devel");

    // Include built gresource
    gtk::gio::resources_register_include!("gresource/ca.potatoe.Kyanite.gresource").unwrap();

    let application = adw::Application::new(id, ApplicationFlags::HANDLES_OPEN);
    application.connect_startup(load_css);
    application.connect_open(open);
    application.connect_activate(activate);

    // Set up accels
    application.set_accels_for_action("win.tab.close", &["<Ctrl>W"]);
    application.set_accels_for_action("win.tab.new_scratch", &["<Ctrl>T"]);
    application.set_accels_for_action("win.tab.new_terminal", &["<Ctrl><Shift>T"]);
    application.set_accels_for_action("win.new_window", &["<Ctrl>N"]);
    application.set_accels_for_action("win.open.file", &["<Ctrl>O"]);
    application.set_accels_for_action("win.open.folder", &["<Ctrl><Shift>O"]);
    application.set_accels_for_action("win.save", &["<Ctrl>S"]);
    application.set_accels_for_action("win.save_as", &["<Ctrl><Shift>S"]);
    application.set_accels_for_action("win.search.toggle", &["<Ctrl>F"]);
    application.set_accels_for_action("win.search.files.toggle", &["<Ctrl><Shift>F"]);
    // run
    application.run();
}

fn activate(app: &Application) {
    let window = EditorWindow::new(app);
    window.present();
}

fn open(app: &Application, files: &[gtk::gio::File], _: &str) {
    let window = EditorWindow::new(app);
    window.present();
    for file in files {
        if let Ok(info) = file.query_info(
            "standard::*",
            FileQueryInfoFlags::empty(),
            Some(&Cancellable::new()),
        ) {
            let file_name = file.path().unwrap().to_str().unwrap().to_string().clone();
            println!("{}", file_name);
            if info.file_type() == gtk::gio::FileType::Directory {
                window.update_folder(&file_name);
                window.imp().split_view.set_show_sidebar(true);
            } else {
                WidgetExt::activate_action(&window, "win.tab.new", Some(&file_name.to_variant()))
                    .unwrap();
            }
        }
    }
}

fn load_css(_: &Application) {
    _load(include_str!("../resources/styles/style.css"));
    _load(include_str!("../resources/styles/completion.css"));
    fn _load(css: &str) {
        let provider = CssProvider::new();
        provider.load_from_string(css);
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
