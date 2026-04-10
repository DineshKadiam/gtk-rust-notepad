use std::any::Any;
use std::str::FromStr;

use adw::TabPage;
use adw::TabView;
use adw::prelude::*;
use adw::subclass::application_window::AdwApplicationWindowImpl;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CustomSorter;
use gtk::DirectoryList;
use gtk::Image;
use gtk::Label;
use gtk::ListItem;
use gtk::ListItemFactory;
use gtk::SignalListItemFactory;
use gtk::SingleSelection;
use gtk::SortListModel;
use gtk::Sorter;
use gtk::TreeExpander;
use gtk::TreeListModel;
use gtk::TreeListRow;
use gtk::TreeListRowSorter;
use gtk::gio::ActionEntry;
use gtk::gio::Cancellable;
use gtk::gio::FileInfo;
use gtk::glib::Variant;
use gtk::glib::VariantTy;
use gtk::glib::clone;
use gtk::glib::{self};
use gtk::{CompositeTemplate, gio};
use sourceview5::prelude::*;

mod imp {

    use adw::{
        OverlaySplitView, StatusPage, TabOverview, TabPage, WindowTitle, ffi::AdwStyleManager,
    };
    use gtk::ListView;

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
                self.window_title.set_subtitle(&page.title().to_string());
            }
            page
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
        // New tab
        let action_new_tab = ActionEntry::builder("tab.new")
            .parameter_type(Some(VariantTy::STRING))
            .activate(move |window: &Self, _, param| {
                let path = param.unwrap().to_string();
                window.imp().new_tab(Some(&path[1..(path.len() - 1)]));
            })
            .build();
        // New scratch buffer
        let action_new_scratch = ActionEntry::builder("tab.new_scratch")
            .parameter_type(None)
            .activate(move |window: &Self, _, _| {
                window.imp().new_tab(None);
            })
            .build();
        // Toggle sidebar
        let action_sidebar_toggle = ActionEntry::builder("sidebar.toggle")
            .parameter_type(None)
            .activate(move |window: &Self, _, _| {
                let split_view = window.imp().split_view.clone();
                split_view.set_show_sidebar(!split_view.shows_sidebar());
            })
            .build();
        // Open file
        let action_open_file = ActionEntry::builder("open.file")
            .parameter_type(None)
            .activate(move |window: &Self, _, _| {
                let gtk_window = (&*window as &dyn Any).downcast_ref::<gtk::Window>();
                let cancellable = Cancellable::new();
                let file_chooser = gtk::FileDialog::new();
                file_chooser.open(
                    gtk_window,
                    Some(&cancellable),
                    clone!(
                        #[strong]
                        window,
                        move |result| {
                            if let Ok(file) = result {
                                let file_name =
                                    file.path().unwrap().to_str().unwrap().to_string().clone();
                                window.imp().new_tab(Some(&file_name));
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
                let gtk_window = (&*window as &dyn Any).downcast_ref::<gtk::Window>();
                let cancellable = Cancellable::new();
                let file_chooser = gtk::FileDialog::new();
                file_chooser.select_folder(
                    gtk_window,
                    Some(&cancellable),
                    clone!(
                        #[strong]
                        window,
                        move |result| {
                            if let Ok(file) = result {
                                let file_name =
                                    file.path().unwrap().to_str().unwrap().to_string().clone();
                                window.update_file_tree(&file_name);
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
            .build(); // Add actions
        self.add_action_entries([
            action_new_tab,
            action_new_scratch,
            action_sidebar_toggle,
            action_open_file,
            action_open_folder,
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
                        &page.title().to_string()
                    } else {
                        ""
                    };
                    window_title.upgrade().unwrap().set_subtitle(subtitle);
                }
            }
        ));
    }

    /// credit to https://codeberg.org/Cyncrovee/Datolite/src/branch/main/src/util.rs which i used as a reference
    /// idk how the fuck to do this and the wiki doesn't really help
    fn update_file_tree(&self, dir_path: &str) {
        let tree = self.imp().file_tree.clone();

        self.imp()
            .sidebar_title
            .set_title(dir_path.split("/").last().unwrap()); // yeah fuck you windows "\" users

        // Sort
        let file_sorter = CustomSorter::new(|a, b| {
            let a = a.downcast_ref::<FileInfo>().unwrap();
            let b = b.downcast_ref::<FileInfo>().unwrap();
            let a_name = a.name().to_str().unwrap().to_string().clone();
            let b_name = b.name().to_str().unwrap().to_string().clone();
            let is_a_hidden = a_name.clone().as_bytes().starts_with(".".as_bytes());
            let is_b_hidden = b_name.clone().as_bytes().starts_with(".".as_bytes());
            if a.file_type() != b.file_type() {
                // 1. Directories go higher than non-directories
                if a.file_type() == gio::FileType::Directory {
                    gtk::Ordering::Smaller
                } else {
                    gtk::Ordering::Larger
                }
            } else if is_a_hidden != is_b_hidden {
                // 2. Hidden files go last.
                if is_a_hidden {
                    gtk::Ordering::Larger
                } else {
                    gtk::Ordering::Smaller
                }
            } else {
                // 3. Compare names
                a_name.cmp(&b_name).into()
            }
        });

        // Create directory list/tree list model
        let list = DirectoryList::new(Some("standard::*"), Some(&gio::File::for_path(dir_path)));
        let list = SortListModel::new(Some(list), Some(file_sorter.clone()));
        let tree_model = TreeListModel::new(list, false, false, move |file_object| {
            if let Some(file_info) = file_object.downcast_ref::<FileInfo>()
                && file_info.file_type() == gio::FileType::Directory
                && let Some(file) = file_info
                    .attribute_object("standard::file")
                    .and_dynamic_cast_ref::<gio::File>()
            {
                let dir_list_local = DirectoryList::new(Some("standard::*"), Some(file));
                let list = SortListModel::new(Some(dir_list_local), Some(file_sorter.clone()));
                Some(list.into())
            } else {
                None
            }
        });

        // Set up factory
        let factory = SignalListItemFactory::new();

        // Adds labels to ListItem objects
        factory.connect_setup(move |_, list_item| {
            // TODO put into template??
            let container = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let label = Label::new(None);
            label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            container.append(&Image::new());
            container.append(&label);
            list_item
                .downcast_ref::<ListItem>()
                .unwrap()
                .set_child(Some(
                    &TreeExpander::builder()
                        .child(&container)
                        .hide_expander(true)
                        .build(),
                ));
        });

        factory.connect_bind(move |_, list_item| {
            let list_item = list_item.downcast_ref::<ListItem>().unwrap();
            if let Some(row) = list_item.item().and_downcast::<TreeListRow>()
                && let Some(file_info) = row.item().and_downcast::<FileInfo>()
                && let Some(tree) = list_item.child().and_downcast::<TreeExpander>()
                && let Some(container) = tree.child().and_downcast::<gtk::Box>()
            {
                let image = container
                    .first_child()
                    .unwrap()
                    .downcast::<Image>()
                    .unwrap();
                let label = container.last_child().unwrap().downcast::<Label>().unwrap();
                if file_info.file_type() == gio::FileType::Directory {
                    label.set_text(&format!("{}/", &file_info.display_name()));
                    // Icons
                    image.set_icon_name(Some("folder-symbolic"));
                    row.connect_expanded_notify(move |row| {
                        image.set_icon_name(Some(if row.is_expanded() {
                            "folder-open-symbolic"
                        } else {
                            "folder-symbolic"
                        }))
                    });
                } else {
                    label.set_text(&file_info.display_name());
                    //file_info.icon().unwrap();
                    image.set_icon_name(Some("text-x-generic-symbolic")); // FIXME dynamic icons when :neofox_pleading:
                }
                tree.set_list_row(Some(&row));
            }
        });

        // Hook things up
        tree.set_model(Some(&SingleSelection::new(Some(tree_model.clone()))));
        tree.set_factory(Some(&factory));
    }
}
