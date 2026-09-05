use std::any::Any;

use adw::{OverlaySplitView, TabPage, TabView, WindowTitle, prelude::*, subclass::prelude::*};
use gtk::{
    CompositeTemplate, gio,
    gio::ActionEntry,
    glib::{self, Properties, VariantTy, clone},
};
use sourceview5::prelude::*;

use crate::{
    action_entry, lsp,
    ui::{buffer::Buffer, dialog::save},
};

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
            action_entry!("new_window", None, new_window),
            action_entry!("open.file", None, open_file),
            action_entry!("open.folder", None, open_folder),
            action_entry!("set_folder", Some(VariantTy::STRING), set_folder),
            action_entry!("reload_config", None, reload_config),
            action_entry!("search.toggle", None, search_toggle),
            action_entry!("search.files.toggle", None, search_files_toggle),
            action_entry!("sidebar.toggle", None, sidebar_toggle),
            action_entry!("show_prefs", None, show_prefs),
            action_entry!("show_recents", None, show_recents),
            action_entry!(
                "status.set_file",
                Some(VariantTy::STRING_ARRAY),
                status_set_file
            ),
            action_entry!("tab.new", Some(VariantTy::STRING), tab_new),
            action_entry!("tab.new_scratch", None, tab_scratch),
            action_entry!("tab.new_terminal", None, tab_term),
            action_entry!("tab.reload", None, tab_reload),
            action_entry!("tab.close", None, tab_close),
            action_entry!("tree.delete_file", None, tree_delete_file),
            action_entry!("tree.rename_file", None, tree_rename),
            action_entry!("tree.new_file", None, tree_new_file),
            action_entry!("tree.open_folder", None, tree_open_folder),
            action_entry!("tree.toggle_expand", None, tree_toggle_expand),
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
            let mut modified_buffers = vec![];
            for page in window.imp().tab_view.pages().iter::<TabPage>() {
                if let Ok(page) = page
                    && let Some(buffer) = page.child().downcast_ref::<Buffer>()
                    && buffer.get_sourceview_buffer().is_modified()
                {
                    modified_buffers.push(buffer.clone());
                }
            }
            if !modified_buffers.is_empty() {
                let dialog = save::new(
                    modified_buffers.clone(),
                    &window.workspace_uri(),
                    || (),
                    clone!(
                        #[strong]
                        window,
                        move || {
                            modified_buffers.iter().for_each(|b| {
                                b.get_sourceview_buffer().set_modified(false);
                            });
                            window.close()
                        }
                    ),
                    clone!(
                        #[strong]
                        window,
                        async move |buf, save| {
                            if save {
                                actions::save_buffer_async(&window, buf.clone()).await;
                            }
                        }
                    ),
                );
                adw::prelude::AdwDialogExt::present(
                    &dialog,
                    Some::<&gtk::Window>(&window.clone().into()),
                );
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
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
                if let Some(buffer) = page.child().downcast_ref::<Buffer>()
                    && buffer.get_sourceview_buffer().is_modified()
                {
                    let binding = window.upgrade().unwrap();
                    let window = binding.obj();
                    let dialog = save::new(
                        vec![buffer.clone()],
                        &window.workspace_uri(),
                        clone!(
                            #[strong(rename_to=tab_view)]
                            window.imp().tab_view,
                            #[strong]
                            page,
                            move || tab_view.close_page_finish(&page, false) // Page close was rejected
                        ),
                        clone!(
                            #[strong(rename_to=tab_view)]
                            window.imp().tab_view,
                            #[strong]
                            page,
                            move || tab_view.close_page_finish(&page, true) // Page close finish
                        ),
                        clone!(
                            #[strong]
                            window,
                            async move |buf, save| {
                                if save {
                                    actions::save_buffer_async(&window, buf.clone()).await;
                                }
                            }
                        ),
                    );
                    adw::prelude::AdwDialogExt::present(
                        &dialog,
                        Some::<&gtk::Window>(&window.clone().into()),
                    );
                    glib::Propagation::Stop
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
        
        // add to recents
        let recents = gtk::RecentManager::default();
        recents.add_item(&format!("file://{dir_path}"));
    }
}
