use gtk::{
    ClosureExpression, CustomFilter, CustomSorter, DirectoryList, Expression, FilterListModel,
    GestureClick, Image, Label, ListItem, ListView, Orientation, PopoverMenu,
    SignalListItemFactory, SingleSelection, SortListModel, StringFilter, TreeExpander,
    TreeListModel, TreeListRow,
    gio::{self, FileInfo},
    glib::{self, clone, closure},
};
use sourceview5::prelude::*;

use crate::config::Config;

/// credit to https://codeberg.org/Cyncrovee/Datolite/src/branch/main/src/util.rs which i used as a reference
/// idk how the fuck to do this and the wiki doesn't really help
pub fn update(
    file_tree_view: &ListView,
    file_list_view: &ListView,
    dir_path: &str,
    file_popover: PopoverMenu,
    folder_popover: PopoverMenu,
) {
    let file_sorter = create_file_sorter();
    let tree_factory = SignalListItemFactory::new();
    let list_factory = SignalListItemFactory::new();

    // Create directory list/tree list model
    let list_model = {
        let directory_list = SortListModel::new(
            Some(DirectoryList::new(
                Some("standard::*"),
                Some(&gio::File::for_path(dir_path)),
            )),
            Some(file_sorter.clone()),
        );
        TreeListModel::new(
            directory_list.clone(),
            false,
            false,
            clone!(
                #[strong]
                file_sorter,
                move |file_object| {
                    if let Some(file_info) = file_object.downcast_ref::<FileInfo>()
                        && file_info.file_type() == gio::FileType::Directory
                        && let Some(file) = file_info
                            .attribute_object("standard::file")
                            .and_dynamic_cast_ref::<gio::File>()
                    {
                        let dir_list_local = DirectoryList::new(Some("standard::*"), Some(file));
                        let list =
                            SortListModel::new(Some(dir_list_local), Some(file_sorter.clone()));
                        Some(list.into())
                    } else {
                        None
                    }
                }
            ),
        )
    };

    // Set up tree factory
    tree_factory.connect_setup(move |_, list_item| {
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
                    .indent_for_icon(false)
                    .margin_start(8)
                    .build(),
            ));
    });
    tree_factory.connect_bind(clone!(
        #[strong(rename_to=list_view)]
        file_tree_view,
        move |_, list_item| {
            let list_item = list_item.downcast_ref::<ListItem>().unwrap();
            if let Some(row) = list_item.item().and_downcast::<TreeListRow>()
                && let Some(file_info) = row.item().and_downcast::<FileInfo>()
                && let Some(tree) = list_item.child().and_downcast::<TreeExpander>()
                && let Some(container) = tree.child().and_downcast::<gtk::Box>()
            {
                let image = container.first_child().and_downcast::<Image>().unwrap();
                let label = container.last_child().and_downcast::<Label>().unwrap();
                if file_info.file_type() == gio::FileType::Directory {
                    label.set_text(&file_info.display_name());
                    // Icons
                    image.set_icon_name(Some("folder-symbolic"));
                    row.connect_expanded_notify(move |row| {
                        image.set_icon_name(Some(if row.is_expanded() {
                            "folder-open-symbolic"
                        } else {
                            "folder-symbolic"
                        }))
                    });
                    // Context menus
                    let tree_click = GestureClick::new();
                    tree_click.set_button(3);
                    tree_click.connect_pressed(clone!(
                        #[strong]
                        container,
                        #[strong]
                        folder_popover,
                        #[strong]
                        list_view,
                        #[strong]
                        list_item,
                        move |gesture, _, _, _| {
                            // Pop up
                            let row_widget = container.parent().unwrap();
                            folder_popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                                0,
                                0,
                                row_widget.size(Orientation::Horizontal),
                                row_widget.size(Orientation::Vertical),
                            )));
                            folder_popover.unparent();
                            folder_popover.set_parent(&row_widget);
                            folder_popover.popup();
                            // Gesture things
                            gesture.set_state(gtk::EventSequenceState::Claimed);
                            // Select item
                            list_view
                                .model()
                                .and_downcast_ref::<SingleSelection>()
                                .unwrap()
                                .set_selected(list_item.position());
                        }
                    ));
                    container.parent().unwrap().add_controller(tree_click);
                } else {
                    label.set_text(&file_info.display_name());
                    if let Some(icon) = file_info.symbolic_icon() {
                        image.set_from_gicon(&icon);
                    }
                    // Context menus
                    let tree_click = GestureClick::new();
                    tree_click.set_button(3);
                    tree_click.connect_pressed(clone!(
                        #[strong]
                        container,
                        #[strong]
                        file_popover,
                        #[strong]
                        list_view,
                        #[strong]
                        list_item,
                        move |gesture, _, _, _| {
                            // Pop up
                            let row_widget = container.parent().unwrap();
                            file_popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                                0,
                                0,
                                row_widget.size(Orientation::Horizontal),
                                row_widget.size(Orientation::Vertical),
                            )));
                            file_popover.unparent();
                            file_popover.set_parent(&row_widget);
                            file_popover.popup();
                            // Gesture things
                            gesture.set_state(gtk::EventSequenceState::Claimed);
                            // Select item
                            list_view
                                .model()
                                .and_downcast_ref::<SingleSelection>()
                                .unwrap()
                                .set_selected(list_item.position());
                        }
                    ));
                    container.parent().unwrap().add_controller(tree_click);
                }
                tree.set_list_row(Some(&row));
            }
        }
    ));

    // Set up list factory
    list_factory.connect_setup(move |_, list_item| {
        // TODO put into template??
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let label = Label::new(None);
        label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        container.set_margin_start(8);
        container.append(&Image::new());
        container.append(&label);
        list_item
            .downcast_ref::<ListItem>()
            .unwrap()
            .set_child(Some(&container));
    });
    list_factory.connect_bind(clone!(move |_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        if let Some(file_info) = list_item.item().and_downcast::<FileInfo>()
            && let Some(container) = list_item.child().and_downcast::<gtk::Box>()
        {
            let image = container.first_child().and_downcast::<Image>().unwrap();
            let label = container.last_child().and_downcast::<Label>().unwrap();
            if file_info.file_type() == gio::FileType::Directory {
                label.set_text(&file_info.display_name());
                image.set_icon_name(Some("folder-symbolic"));
            } else {
                label.set_text(&file_info.display_name());
                if let Some(icon) = file_info.symbolic_icon() {
                    image.set_from_gicon(&icon);
                }
            }
        }
    }));

    // Hook things up
    file_tree_view.set_model(Some(&SingleSelection::new(Some(FilterListModel::new(
        Some(list_model),
        None::<gtk::Filter>,
    )))));
    file_tree_view.set_factory(Some(&tree_factory));
    file_list_view.set_factory(Some(&list_factory));
    set_filter_hidden_files(
        &file_tree_view,
        Config::get_or_default().sidebar.show_hidden,
    );
}

/// Filter out hidden files on the file tree
pub fn set_filter_hidden_files(file_tree_view: &ListView, show_hidden: bool) {
    let filter = CustomFilter::new(move |object| {
        if let Some(row) = object.downcast_ref::<TreeListRow>()
            && let Some(file_info) = row.item().and_downcast::<FileInfo>()
        {
            !file_info.is_hidden()
        } else {
            true
        }
    });
    if let Some(selection_model) = file_tree_view.model().and_downcast_ref::<SingleSelection>()
        && let Some(filter_model) = selection_model
            .model()
            .and_downcast_ref::<FilterListModel>()
    {
        filter_model.set_filter(if !show_hidden { Some(&filter) } else { None });
    }
}

pub fn set_search_list(file_list_view: &ListView, dir_path: &str) {
    if file_list_view.model().is_some() {
        return;
    }

    // Create expanded list model for the list
    let list_model_expanded = {
        let file_sorter = create_file_sorter();
        let directory_list = SortListModel::new(
            Some(DirectoryList::new(
                Some("standard::*"),
                Some(&gio::File::for_path(dir_path)),
            )),
            Some(file_sorter.clone()),
        );
        TreeListModel::new(directory_list, true, true, move |file_object| {
            if let Some(file_info) = file_object.downcast_ref::<FileInfo>()
                && file_info.file_type() == gio::FileType::Directory
                && !file_info
                    .name()
                    .to_str()
                    .unwrap_or_default()
                    .starts_with(".")
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
        })
    };

    // Filter stuff
    // This took an absurdly long time to figure out. Thanks, no documentation! Also fuck you :)
    let params: [Expression; 0] = [];
    let expression = ClosureExpression::new::<String>(
        params,
        closure!(|object: glib::Object| {
            if let Some(file_info) = object.downcast_ref::<FileInfo>() {
                let name = file_info.name().to_str().unwrap().to_string().clone();
                Some(name)
            } else {
                Some(String::new())
            }
        }),
    );

    let filter = StringFilter::new(Some(expression));
    let filter_model = FilterListModel::new(Some(list_model_expanded.clone()), Some(filter));

    filter_model.set_incremental(true);
    file_list_view.set_model(Some(&SingleSelection::new(Some(filter_model))));
}

pub fn clear_search_list(file_list_view: &ListView) {
    file_list_view.set_model(None::<&SingleSelection>);
}

/// Create a CustomSorter that sorts based on filename, sorting folders before files.
fn create_file_sorter() -> CustomSorter {
    CustomSorter::new(|a, b| {
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
    })
}
