use std::any::Any;

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::{OverlaySplitView, StatusPage, TabOverview, TabPage, TabView, WindowTitle};
use gtk::gio::{ActionEntry, Cancellable, FileInfo};
use gtk::glib::{self, Properties, VariantTy, clone};
use gtk::{CompositeTemplate, ListView, TreeListRow, gdk, gio};
use sourceview5::prelude::*;

use crate::ui::file_tree;

mod imp {

    use gtk::{SearchBar, SearchEntry};

    use crate::ui::buffer::Buffer;

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/window.blp")]
    #[properties(wrapper_type = super::EditorWindow)]
    pub struct EditorWindow {
        #[template_child]
        pub tabview: TemplateChild<TabView>,
        #[template_child]
        pub tab_overview: TemplateChild<TabOverview>,
        #[template_child]
        pub split_view: TemplateChild<OverlaySplitView>,
        #[template_child]
        pub window_title: TemplateChild<WindowTitle>,
        #[template_child]
        pub sidebar_title: TemplateChild<WindowTitle>,
        #[template_child]
        pub startpage: TemplateChild<StatusPage>,
        #[template_child]
        pub file_tree: TemplateChild<ListView>,
        #[template_child]
        pub search_bar: TemplateChild<SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<SearchEntry>,
    }

    impl EditorWindow {
        fn tab_name(full_path: &str, modified: bool) -> String {
            let modified_string = if modified { " [+]" } else { "" };
            let path_string = if full_path != "" {
                full_path.split("/").last().unwrap()
            } else {
                "scratch"
            }
            .to_string();
            path_string + modified_string
        }

        pub fn new_tab(&self, path: Option<&str>) -> TabPage {
            // Get rid of welcome page
            if self.startpage.parent().is_some() {
                let page = self.tabview.page(&self.startpage.get());
                self.tabview.close_page(&page);
            }
            // Buffer
            let buffer = sourceview5::Buffer::new(None);
            buffer.set_highlight_syntax(true);
            //     sourceview5::StyleSchemeManager::new().scheme_ids()
            if let Some(ref scheme) = sourceview5::StyleSchemeManager::new().scheme("Adwaita-dark")
                && adw::StyleManager::default().is_dark()
            {
                buffer.set_style_scheme(Some(scheme));
            }
            // Create buffer UI and page
            let buffer_ui = Buffer::with_source(buffer.clone());
            let page = self.tabview.add_page(&buffer_ui, None);
            buffer_ui.set_file_path(path.unwrap_or_default());
            // Files
            if let Some(path) = path {
                let file = gio::File::for_path(path);
                let file = sourceview5::File::builder().location(&file).build();
                let loader = sourceview5::FileLoader::new(&buffer, &file);
                loader.load_async(glib::Priority::default(), gio::Cancellable::NONE, |_| {});
                // Set syntax highlighting
                if let Some(ref language) =
                    sourceview5::LanguageManager::new().guess_language(Some(&path), None)
                {
                    buffer.set_language(Some(language));
                }
            }
            // Add tab
            page.set_title(&EditorWindow::tab_name(&path.unwrap_or_default(), false));
            if page.is_selected() {
                self.window_title.set_title(&page.title());
            }
            // Connect modified
            //buffer_ui.imp().source_view.connect_paste_clipboard(f)
            buffer.connect_changed(clone!(
                #[strong]
                page,
                #[strong(rename_to=window_title)]
                self.window_title,
                move |buffer| {
                    if !buffer.is_loading() {
                        page.set_title(&EditorWindow::tab_name(&buffer_ui.file_path(), true));
                        if page.is_selected() {
                            window_title.set_title(&page.title());
                        }
                    }
                }
            ));
            // Return page
            page
        }

        pub fn save_current_buffer(&self, save_as: bool) {
            if let Some(page) = self.tabview.selected_page()
                && let Some(buffer) = page.child().downcast_ref::<Buffer>()
            {
                if buffer.file_path() == "" || save_as {
                    gtk::FileDialog::new().save(
                        Some::<&gtk::Window>(&self.obj().clone().into()),
                        Some(&Cancellable::new()),
                        clone!(
                            #[strong]
                            buffer,
                            #[strong]
                            page,
                            #[weak(rename_to=window)]
                            self,
                            move |result| {
                                if let Ok(file) = result {
                                    let file_name =
                                        file.path().unwrap().to_str().unwrap().to_string().clone();
                                    // Set syntax highlighting
                                    if let Some(ref language) = sourceview5::LanguageManager::new()
                                        .guess_language(Some(&file_name), None)
                                    {
                                        buffer
                                            .imp()
                                            .source_view
                                            .buffer()
                                            .downcast_ref::<sourceview5::Buffer>()
                                            .unwrap()
                                            .set_language(Some(language));
                                    }
                                    // Set window/tab title
                                    page.set_title(&EditorWindow::tab_name(
                                        &buffer.file_path(),
                                        false,
                                    ));
                                    if page.is_selected() {
                                        // TODO clearly i should be able to simplify this right
                                        window.window_title.set_title(&page.title());
                                    }
                                    // Set buffer path and save
                                    buffer.set_file_path(file_name);
                                    EditorWindow::save_buffer(&buffer);
                                }
                            }
                        ),
                    );
                } else {
                    EditorWindow::save_buffer(buffer);
                    page.set_title(&EditorWindow::tab_name(&buffer.file_path(), false));
                    if page.is_selected() {
                        self.window_title.set_title(&page.title());
                    }
                }
            }
        }

        fn save_buffer(buffer: &Buffer) {
            let file = gio::File::for_path(buffer.file_path());
            let file = sourceview5::File::builder().location(&file).build();
            let save = sourceview5::FileSaver::new(
                buffer
                    .imp()
                    .source_view
                    .buffer()
                    .downcast_ref::<sourceview5::Buffer>()
                    .unwrap(),
                &file,
            );
            save.save_async(glib::Priority::default(), gio::Cancellable::NONE, |_| {});
        }

        fn connect_tree(&self) {
            self.file_tree.connect_activate(|list, idx| {
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
        }
    }

    impl From<EditorWindow> for gtk::Window {
        fn from(val: EditorWindow) -> Self {
            (&val as &dyn Any)
                .downcast_ref::<gtk::Window>()
                .unwrap()
                .clone()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EditorWindow {
        const NAME: &'static str = "EditorWindow";
        type Type = super::EditorWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EditorWindow {
        fn constructed(&self) {
            self.parent_constructed();
            self.connect_tree();
            self.obj().setup_actions();
            self.obj().setup_callbacks();
            self.obj()
                .update_file_tree(std::env::home_dir().unwrap().to_str().unwrap());
        }
    }
    impl WidgetImpl for EditorWindow {}
    impl WindowImpl for EditorWindow {}
    impl ApplicationWindowImpl for EditorWindow {}
    impl AdwApplicationWindowImpl for EditorWindow {}
}

glib::wrapper! {
    pub struct EditorWindow(ObjectSubclass<imp::EditorWindow>)
        @extends adw::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionMap, gio::ActionGroup, gtk::ConstraintTarget, gtk::Accessible, gtk::Buildable, gtk::Native, gtk::Root, gtk::ShortcutManager, gtk::ApplicationWindow;
}

impl EditorWindow {
    pub fn new(app: &adw::Application) -> Self {
        glib::Object::builder::<Self>()
            .property("application", app)
            .build()
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
                                window.update_file_tree(file.path().unwrap().to_str().unwrap());
                                // close tabs + open sidebar
                                window.imp().split_view.set_show_sidebar(true);
                                for page in window.imp().tabview.pages().iter::<TabPage>() {
                                    window.imp().tabview.close_page(&page.unwrap());
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
            // Closes current tab
            ActionEntry::builder("tab.close")
                .parameter_type(None)
                .activate(move |window: &Self, _, _param| {
                    if let Some(page) = window.imp().tabview.selected_page() {
                        window.imp().tabview.close_page(&page);
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
                    if let Some(page) = window.imp().tabview.selected_page()
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
        self.imp().tabview.connect_selected_page_notify(clone!(
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
                        "spikedit"
                    };
                    if window.search_bar.is_search_mode() {
                        tab_view.activate_action("win.search.toggle", None).unwrap();
                    }
                    window_title.upgrade().unwrap().set_title(subtitle);
                }
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
                    if let Some(page) = window.upgrade().unwrap().tabview.selected_page()
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
                if let Some(page) = window.upgrade().unwrap().tabview.selected_page()
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
                if let Some(page) = window.upgrade().unwrap().tabview.selected_page()
                    && let Some(buffer) = page.child().downcast_ref::<crate::ui::buffer::Buffer>()
                {
                    match buffer.search(&entry.text(), crate::ui::buffer::SearchDirection::Cursor) {
                        Ok(_) => entry.remove_css_class("error"),
                        Err(_) => entry.add_css_class("error"),
                    }
                }
            }
        ));
    }

    fn update_file_tree(&self, dir_path: &str) {
        self.imp()
            .sidebar_title
            .set_title(dir_path.split("/").last().unwrap()); // yeah fuck you windows "\" users

        file_tree::update(&self.imp().file_tree.clone(), dir_path);
    }
}
