use std::any::Any;

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::{OverlaySplitView, StatusPage, TabOverview, TabPage, TabView, WindowTitle};
use gtk::gio::{ActionEntry, Cancellable, FileInfo};
use gtk::glib::{self, Properties, VariantTy, clone};
use gtk::{
    CompositeTemplate, ListView,
    TreeListRow, gio,
};
use sourceview5::prelude::*;

use crate::ui::file_tree;

mod imp {

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
    }

    impl EditorWindow {
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
            let buffer_ui = Buffer::with_source(buffer);
            let page = self.tabview.add_page(&buffer_ui, None);
            buffer_ui.set_file_path(path.unwrap_or_default());
            page.set_title(&if let Some(path) = path {
                let file = gio::File::for_path(path);
                file.basename()
                    .unwrap()
                    .as_mut_os_string()
                    .clone()
                    .into_string()
                    .unwrap()
            } else {
                "Scratch buffer".to_string()
            });
            if page.is_selected() {
                self.window_title.set_title(&page.title());
            }
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
                                    let file_base_name = file_name.split("/").last().unwrap();
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
                                    page.set_title(file_base_name);
                                    if page.is_selected() {
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
            self.obj().update_file_tree("/home/pastthepixels"); // FIXME
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
        ]);
    }

    fn setup_callbacks(&self) {
        let window = self.imp().downgrade();
        let window_title = self.imp().window_title.downgrade();
        self.imp()
            .tab_overview
            .connect_create_tab(move |_| window.upgrade().unwrap().new_tab(None));
        self.imp().tabview.connect_selected_page_notify(clone!(
            #[strong]
            window_title,
            move |tab_view| {
                {
                    let subtitle = if let Some(page) = tab_view.selected_page() {
                        &page.title()
                    } else {
                        "spikedit"
                    };
                    window_title.upgrade().unwrap().set_title(subtitle);
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
