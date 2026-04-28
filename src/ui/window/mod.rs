use std::any::Any;

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::{OverlaySplitView, TabOverview, TabPage, TabView, WindowTitle};
use gtk::gio::{ActionEntry, Cancellable, FileInfo};
use gtk::glib::{self, Properties, Variant, VariantTy, clone};
use gtk::{CompositeTemplate, Label, ListView, StackPage, TreeListRow, gio};
use sourceview5::prelude::*;

use crate::ui::file_tree;
use crate::ui::preferences::PreferencesDialog;
use crate::{lsp, utils};

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
        // Open file
        let action_open_file = ActionEntry::builder("open.file")
            .parameter_type(None)
            .activate(move |window: &Self, _, _| {
                gtk::FileDialog::new().open(
                    window.into(),
                    Some(&Cancellable::new()),
                    clone!(
                        #[strong]
                        window,
                        move |result| {
                            if let Ok(file) = result {
                                window
                                    .imp()
                                    .new_tab(Some(file.path().unwrap().to_str().unwrap()));
                            }
                        }
                    ),
                );
            })
            .build();
        // Open folder
        let action_open_folder = ActionEntry::builder("open.folder")
            .parameter_type(None)
            .activate(move |window: &Self, _, _| {
                gtk::FileDialog::new().select_folder(
                    window.into(),
                    Some(&Cancellable::new()),
                    clone!(
                        #[strong]
                        window,
                        move |result| {
                            if let Ok(file) = result {
                                window.update_folder(file.path().unwrap().to_str().unwrap());
                                // close tabs + open sidebar
                                window.imp().split_view.set_show_sidebar(true);
                                for page in window.imp().tab_view.pages().iter::<TabPage>() {
                                    window.imp().tab_view.close_page(&page.unwrap());
                                }
                            }
                        }
                    ),
                );
            })
            .build();
        // Add actions
        self.add_action_entries([
            // Loads a file into a new tab
            ActionEntry::builder("tab.new")
                .parameter_type(Some(VariantTy::STRING))
                .activate(move |window: &Self, _, param| {
                    let path = param.unwrap().to_string();
                    window.imp().new_tab(Some(&path[1..(path.len() - 1)]));
                })
                .build(),
            // New empty tab -- scratch buffer
            ActionEntry::builder("tab.new_scratch")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| {
                    window.imp().new_tab(None);
                })
                .build(),
            // New terminal tab
            ActionEntry::builder("tab.new_terminal")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| {
                    window.imp().new_terminal_tab();
                })
                .build(),
            // Closes current tab
            ActionEntry::builder("tab.close")
                .parameter_type(None)
                .activate(move |window: &Self, _, _param| {
                    if let Some(page) = window.imp().tab_view.selected_page() {
                        // LSP file close
                        if let Some(buffer) = page.child().downcast_ref::<super::buffer::Buffer>()
                            && !buffer.file_path().is_empty()
                        {
                            lsp::file::close(&buffer.file_path(), &window.workspace_uri());
                        }
                        window.imp().tab_view.close_page(&page);
                    }
                })
                .build(),
            // Toggles sidebar visibility
            ActionEntry::builder("sidebar.toggle")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| {
                    let split_view = window.imp().split_view.clone();
                    split_view.set_show_sidebar(!split_view.shows_sidebar());
                })
                .build(),
            action_open_file,
            action_open_folder,
            // Saves the buffer in the current tab
            ActionEntry::builder("save")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| window.imp().save_current_buffer(false))
                .build(),
            // Saves the buffer in the current tab to a specified file
            ActionEntry::builder("save_as")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| window.imp().save_current_buffer(true))
                .build(),
            // Show/hide search bar
            ActionEntry::builder("search.toggle")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| {
                    if let Some(page) = window.imp().tab_view.selected_page()
                        && let Some(buffer) = page.child().downcast_ref::<super::buffer::Buffer>()
                    {
                        let mode = !window.imp().search_bar.is_search_mode();
                        window.imp().search_bar.set_search_mode(mode);
                        // Focus the search entry (text widget)
                        if mode {
                            window.imp().search_entry.set_text("");
                            window.imp().search_entry.grab_focus();
                        } else {
                            // Focus the current tab
                            buffer.imp().source_view.grab_focus();
                        }
                    }
                })
                .build(),
            // New window
            ActionEntry::builder("new_window")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| {
                    let app = window
                        .application()
                        .unwrap()
                        .downcast_ref::<adw::Application>()
                        .unwrap()
                        .clone();
                    let new_window = Self::new(&app);
                    new_window.present();
                })
                .build(),
            // Shows prefs dialog
            ActionEntry::builder("show_prefs")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| {
                    let prefs = PreferencesDialog::new();
                    prefs.present(&window.clone().into());
                })
                .build(),
            // Reload config
            ActionEntry::builder("reload_config")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| {
                    window.imp().reload_config();
                })
                .build(),
            // Set statusbar files segment
            ActionEntry::builder("status.set_file")
                .parameter_type(Some(VariantTy::TUPLE))
                .activate(move |window: &Self, _, param| {
                    let param = param.unwrap();
                    let file_path = glib::Variant::get::<String>(&param.child_value(0)).unwrap();
                    let file_type = glib::Variant::get::<String>(&param.child_value(1)).unwrap();
                    let is_shown: bool = param.child_value(2).get::<bool>().unwrap();
                    utils::ui::get_child_by_name::<Label>(
                        window.imp().status_files.get(),
                        "status_file_path",
                    )
                    .unwrap()
                    .set_label(&file_path);
                    utils::ui::get_child_by_name::<Label>(
                        window.imp().status_files.get(),
                        "status_lang",
                    )
                    .unwrap()
                    .set_label(&file_type);
                    window.imp().status_files.set_visible(is_shown);
                })
                .build(),
            // CD to folder in tree
            ActionEntry::builder("tree.open_folder")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| {
                    if let Some(selected) = window
                        .imp()
                        .file_tree
                        .model()
                        .and_downcast_ref::<gtk::SingleSelection>()
                        .unwrap()
                        .selected_item()
                        && let Some(row) = selected.downcast_ref::<TreeListRow>()
                        && let Some(file_info) = row.item().and_downcast::<FileInfo>()
                        && let Some(file) = file_info
                            .attribute_object("standard::file")
                            .and_downcast_ref::<gio::File>()
                        && let Some(path) = file
                            .path()
                            .map(|a| a.to_str().unwrap_or_default().to_string())
                    {
                        window.update_folder(&path);
                    }
                })
                .build(),
            // CD to folder in tree
            ActionEntry::builder("tree.toggle_expand")
                .parameter_type(None)
                .activate(move |window: &Self, _, _| {
                    if let Some(selected) = window
                        .imp()
                        .file_tree
                        .model()
                        .and_downcast_ref::<gtk::SingleSelection>()
                        .unwrap()
                        .selected_item()
                        && let Some(row) = selected.downcast_ref::<TreeListRow>()
                        && row.is_expandable()
                    {
                        row.set_expanded(!row.is_expanded());
                    }
                })
                .build(),
        ]);
    }

    fn setup_callbacks(&self) {
        let window = self.imp().downgrade();
        let window_title = self.imp().window_title.downgrade();
        self.imp().tab_overview.connect_create_tab(clone!(
            #[strong]
            window,
            move |_| window.upgrade().unwrap().new_tab(None)
        ));
        self.imp().tab_view.connect_n_pages_notify(clone!(
            #[strong(rename_to=stack)]
            self.imp().stack,
            move |tab_view| {
                if tab_view.n_pages() == 0 {
                    stack.set_visible_child_name("empty");
                    // Hide file status
                    let variant = Variant::tuple_from_iter(vec![
                        "".to_variant(),
                        "".to_variant(),
                        false.to_variant(),
                    ]);
                    tab_view
                        .activate_action("win.status.set_file", Some(&variant))
                        .unwrap();
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
        self.imp().file_tree.connect_activate(|list, idx| {
            if let Some(model) = list.model()
                && let Some(item_obj) = model.item(idx)
                && let Some(item) = item_obj.downcast_ref::<TreeListRow>()
            {
                if item.is_expandable() {
                    // it's a folder
                    item.set_expanded(!item.is_expanded());
                } else if let Some(file_info) = item.item().and_downcast::<FileInfo>() {
                    // it's a file
                    let file = file_info
                        .attribute_object("standard::file")
                        .and_downcast_ref::<gio::File>()
                        .unwrap()
                        .clone();
                    let file_name = file.path().unwrap().to_str().unwrap().to_string().clone();
                    list.activate_action("win.tab.new", Some(&file_name.into()))
                        .unwrap();
                }
            };
        });
        self.imp().file_list.connect_activate(clone!(
            #[strong]
            window,
            move |list, idx| {
                if let Some(model) = list.model()
                    && let Some(item_obj) = model.item(idx)
                    && let Some(item) = item_obj.downcast_ref::<FileInfo>()
                {
                    let file = item
                        .attribute_object("standard::file")
                        .and_downcast_ref::<gio::File>()
                        .unwrap()
                        .clone();
                    let file_name = file.path().unwrap().to_str().unwrap().to_string().clone();
                    let window = window.upgrade().unwrap();
                    window.file_stack.set_visible_child_name("tree_page");
                    window.file_search.set_text("");
                    if item.file_type() == gio::FileType::Directory {
                        window.obj().update_folder(&file_name);
                    } else {
                        list.activate_action("win.tab.new", Some(&file_name.into()))
                            .unwrap();
                    }
                };
            }
        ));
        // File search
        self.imp().file_search.connect_search_changed(clone!(
            #[strong]
            window,
            move |search| {
                let window = window.upgrade().unwrap();

                window
                    .file_stack
                    .set_visible_child_name(if search.text().is_empty() {
                        "tree_page"
                    } else {
                        "list_page"
                    });

                if search.text().is_empty() {
                    file_tree::clear_search_list(&window.file_list);
                } else {
                    file_tree::set_search_list(
                        &window.file_list,
                        &window.workspace_uri.borrow().to_string(),
                    );
                    // Holy fuck
                    window
                        .file_list
                        .model()
                        .and_downcast_ref::<gtk::SingleSelection>()
                        .unwrap()
                        .model()
                        .and_downcast_ref::<gtk::FilterListModel>()
                        .unwrap()
                        .filter()
                        .and_downcast_ref::<gtk::StringFilter>()
                        .unwrap()
                        .set_search(Some(&search.text()));
                }
            }
        ));
        self.imp().file_search.connect_activate(clone!(
            #[strong]
            window,
            move |_| {
                window.upgrade().unwrap().file_list.grab_focus();
            }
        ));
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
        self.imp()
            .sidebar_title
            .set_title(dir_path.split("/").last().unwrap()); // yeah fuck you windows "\" users

        file_tree::update(
            &self.imp().file_tree.clone(),
            &self.imp().file_list.clone(),
            dir_path,
        );

        self.imp().terminal_view.setup(dir_path);

        self.set_property("workspace_uri", dir_path.to_string());
    }
}
