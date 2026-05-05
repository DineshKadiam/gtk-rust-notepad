use std::any::Any;

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::{OverlaySplitView, TabPage, TabView, WindowTitle};
use gtk::gio::ActionEntry;
use gtk::glib::{self, Properties, VariantTy, clone};
use gtk::{CompositeTemplate, gio};
use sourceview5::prelude::*;

use crate::ui::buffer::Buffer;
use crate::{lsp, new_entry};

mod actions;
mod imp;
mod tabs;

glib::wrapper! {
    pub struct EditorWindow(ObjectSubclass<imp::EditorWindow>)
        @extends adw::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionMap, gio::ActionGroup, gtk::ConstraintTarget, gtk::Accessible, gtk::Buildable, gtk::Native, gtk::Root, gtk::ShortcutManager, gtk::ApplicationWindow;
}

impl EditorWindow {
    pub fn new(app: &adw::Application) -> Self {
        let win = glib::Object::builder::<Self>()
            .property("application", app)
            .build();
        // Do debug things
        #[cfg(debug_assertions)]
        {
            win.add_css_class("devel");
        }
        win
    }

    fn setup_actions(&self) {
        self.add_action_entries([
            new_entry!("new_window", None, new_window),
            new_entry!("open.file", None, open_file),
            new_entry!("open.folder", None, open_folder),
            new_entry!("reload_config", None, reload_config),
            new_entry!("search.toggle", None, search_toggle),
            new_entry!("search.files.toggle", None, search_files_toggle),
            new_entry!("sidebar.toggle", None, sidebar_toggle),
            new_entry!("show_prefs", None, show_prefs),
            new_entry!(
                "status.set_file",
                Some(VariantTy::STRING_ARRAY),
                status_set_file
            ),
            new_entry!("tab.new", Some(VariantTy::STRING), tab_new),
            new_entry!("tab.new_scratch", None, tab_scratch),
            new_entry!("tab.new_terminal", None, tab_term),
            new_entry!("tab.close", None, tab_close),
            new_entry!("tree.delete_file", None, tree_delete_file),
            new_entry!("tree.new_file", None, tree_new_file),
            new_entry!("tree.open_folder", None, tree_open_folder),
            new_entry!("tree.toggle_expand", None, tree_toggle_expand),
            // Saves the buffer in the current tab
            ActionEntry::builder("save")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| actions::save_current_buffer(window, false))
                .build(),
            // Saves the buffer in the current tab to a specified file
            ActionEntry::builder("save_as")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| actions::save_current_buffer(window, true))
                .build(),
        ]);
    }

    fn setup_callbacks(&self) {
        let window = self.imp().downgrade();
        let window_title = self.imp().window_title.downgrade();
        self.connect_close_request(|window| {
            for page in window.imp().tab_view.pages().iter::<TabPage>() {
                if let Ok(page) = page
                    && let Some(buffer) = page.child().downcast_ref::<Buffer>()
                    && buffer.get_sourceview_buffer().is_modified()
                {
                    window.imp().tab_view.close_page(&page);
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        self.imp().tab_view.connect_n_pages_notify(clone!(
            #[strong(rename_to=stack)]
            self.imp().stack,
            move |tab_view| {
                if tab_view.n_pages() == 0 {
                    stack.set_visible_child_name("empty");
                    // Hide file status
                    tab_view
                        .activate_action(
                            "win.status.set_file",
                            Some(&vec!["".to_variant(), "".to_variant(), "".to_variant()].into()),
                        )
                        .unwrap();
                }
            }
        ));
        self.imp().tab_view.connect_close_page(clone!(
            #[strong]
            window,
            move |_, page| {
                if let Some(buffer) = page.child().downcast_ref::<Buffer>() {
                    match buffer.close_buffer(window.upgrade().unwrap().obj().clone(), page.clone())
                    {
                        true => glib::Propagation::Proceed,
                        false => glib::Propagation::Stop,
                    }
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));
        self.imp().tab_view.connect_selected_page_notify(clone!(
            #[strong]
            window_title,
            #[strong]
            window,
            move |tab_view| {
                {
                    let window = window.upgrade().unwrap();
                    let subtitle = if let Some(page) = tab_view.selected_page() {
                        &page.title()
                    } else {
                        "Kyanite"
                    };
                    if window.search_bar.is_search_mode() {
                        tab_view.activate_action("win.search.toggle", None).unwrap();
                    }
                    window_title.upgrade().unwrap().set_title(subtitle);
                    // For buffers
                    if let Some(page) = tab_view.selected_page()
                        && let Some(buffer) = page.child().downcast_ref::<super::buffer::Buffer>()
                    {
                        buffer.set_status(&window.workspace_uri.borrow());
                    }
                }
            }
        ));
        self.imp().tab_view.connect_create_window(clone!(
            #[strong]
            window,
            move |_| {
                let window = window.upgrade().unwrap();
                let app = window
                    .obj()
                    .application()
                    .and_downcast_ref::<adw::Application>()
                    .unwrap()
                    .clone();
                let new_window = Self::new(&app);
                // Get rid of welcome page
                new_window.imp().stack.set_visible_child_name("tabs");
                new_window.present();
                new_window
                    .imp()
                    .tab_view
                    .downcast_ref::<adw::TabView>()
                    .cloned()
            }
        ));
        // Custom search keybinds
        let search_controller = gtk::EventControllerKey::new();
        search_controller.connect_key_pressed(clone!(
            #[strong(rename_to=search_entry)]
            self.imp().search_entry,
            #[strong]
            window,
            move |_, key, _, modifier| {
                if modifier == gtk::gdk::ModifierType::SHIFT_MASK && key == gtk::gdk::Key::Return {
                    // Reverse search -- shift+Enter
                    if let Some(page) = window.upgrade().unwrap().tab_view.selected_page()
                        && let Some(buffer) =
                            page.child().downcast_ref::<crate::ui::buffer::Buffer>()
                    {
                        match buffer.search(
                            &search_entry.text(),
                            crate::ui::buffer::SearchDirection::Back,
                        ) {
                            Ok(_) => search_entry.remove_css_class("error"),
                            Err(_) => search_entry.add_css_class("error"),
                        }
                    }
                }
                glib::Propagation::Proceed
            }
        ));
        self.imp().search_entry.add_controller(search_controller);
        self.imp().search_entry.connect_activate(clone!(
            #[strong]
            window,
            move |entry| {
                if let Some(page) = window.upgrade().unwrap().tab_view.selected_page()
                    && let Some(buffer) = page.child().downcast_ref::<crate::ui::buffer::Buffer>()
                {
                    match buffer.search(&entry.text(), crate::ui::buffer::SearchDirection::Forward)
                    {
                        Ok(_) => entry.remove_css_class("error"),
                        Err(_) => entry.add_css_class("error"),
                    }
                }
            }
        ));
        self.imp().search_entry.connect_search_changed(clone!(
            #[strong]
            window,
            move |entry| {
                if let Some(page) = window.upgrade().unwrap().tab_view.selected_page()
                    && let Some(buffer) = page.child().downcast_ref::<crate::ui::buffer::Buffer>()
                {
                    match buffer.search(&entry.text(), crate::ui::buffer::SearchDirection::Cursor) {
                        Ok(_) => entry.remove_css_class("error"),
                        Err(_) => entry.add_css_class("error"),
                    }
                }
            }
        ));
        // File tree
        self.imp().sidebar.connect_all(&self.clone());
        // Switching to/from dark theme
        let style_man = adw::StyleManager::default();
        style_man.connect_dark_notify(clone!(
            #[strong]
            window,
            move |_| {
                window.upgrade().unwrap().reload_config();
            }
        ));
        // Set up tab context thingies
        tabs::init_tab_menus(self);
    }

    pub fn update_folder(&self, dir_path: &str) {
        self.imp().sidebar.update_folder(dir_path);

        self.imp().terminal_view.setup(dir_path);

        self.set_property("workspace_uri", dir_path.to_string());
    }
}
