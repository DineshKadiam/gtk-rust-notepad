use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::TreeExpander;
use gtk::TreeListRow;
use gtk::gio;
use gtk::gio::FileInfo;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;

use crate::config::Config;
use crate::ui::window::EditorWindow;
use crate::utils;

mod file_tree;
pub mod icons;

mod imp {

    use adw::WindowTitle;
    use gtk::{ListView, PopoverMenu, SearchEntry, Stack};

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/sidebar.blp")]
    #[properties(wrapper_type = super::Sidebar)]
    pub struct Sidebar {
        #[template_child]
        pub file_tree: TemplateChild<ListView>,
        #[template_child]
        pub file_list: TemplateChild<ListView>,
        #[template_child]
        pub file_search: TemplateChild<SearchEntry>,
        #[template_child]
        pub file_stack: TemplateChild<Stack>,
        #[template_child]
        pub folder_popover: TemplateChild<PopoverMenu>,
        #[template_child]
        pub file_popover: TemplateChild<PopoverMenu>,
        #[template_child]
        pub sidebar_title: TemplateChild<WindowTitle>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Sidebar {
        const NAME: &'static str = "Sidebar";
        type Type = super::Sidebar;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Sidebar {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for Sidebar {}
    impl BinImpl for Sidebar {}
}

glib::wrapper! {
pub struct Sidebar(ObjectSubclass<imp::Sidebar>)
    @extends adw::Bin, gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Sidebar {
    /// Returns the selected folder from the TreeView
    pub fn get_selected_folder(&self) -> Option<String> {
        if let Some(selected) = self
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
        {
            file.path()
                .map(|a| a.to_str().unwrap_or_default().to_string())
        } else {
            None
        }
    }

    /// Returns the selected widget from the TreeView
    pub fn get_selected_widget(&self) -> Option<gtk::Widget> {
        if let Some(selected) = self
            .imp()
            .file_tree
            .model()
            .and_downcast_ref::<gtk::SingleSelection>()
            .unwrap()
            .selected_item()
            .and_downcast_ref::<gtk::ListItem>()
            && let Some(widget) = selected.child().and_downcast::<TreeExpander>()
        {
            Some(widget.clone().into())
        } else {
            None
        }
    }

    /// Toggle expand/collapse of a folder in the file tree
    pub fn tree_toggle_expand(&self) {
        if let Some(selected) = self
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
    }

    pub fn connect_all(&self, window: &EditorWindow) {
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
            #[weak(rename_to=this)]
            self,
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
                    this.imp().file_stack.set_visible_child_name("tree_page");
                    this.imp().file_search.set_text("");
                    if item.file_type() == gio::FileType::Directory {
                        window.update_folder(&file_name);
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
            #[strong(rename_to=file_list)]
            self.imp().file_list,
            #[strong(rename_to=file_stack)]
            self.imp().file_stack,
            move |search| {
                file_stack.set_visible_child_name(if search.text().is_empty() {
                    "tree_page"
                } else {
                    "list_page"
                });

                if search.text().is_empty() {
                    file_tree::clear_search_list(&file_list);
                } else {
                    file_tree::set_search_list(
                        &file_list,
                        &window.imp().workspace_uri.borrow().to_string(),
                    );
                    // Holy fuck
                    file_list
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
            #[strong(rename_to=file_list)]
            self.imp().file_list,
            move |_| {
                file_list.grab_focus();
            }
        ));
    }

    pub fn update_folder(&self, dir_path: &str) {
        self.imp()
            .sidebar_title
            .set_title(utils::basename(dir_path));

        file_tree::update(
            &self.imp().file_tree.clone(),
            &self.imp().file_list.clone(),
            dir_path,
            self.imp().file_popover.clone(),
            self.imp().folder_popover.clone(),
        );
    }

    pub fn reload_config(&self) {
        file_tree::set_filter_hidden_files(
            &self.imp().file_tree,
            Config::get_or_default().sidebar.show_hidden,
        );
    }
}
