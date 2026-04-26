use gtk::{
    Button, CustomSorter, DirectoryList, GestureClick, Image, Label, ListItem, ListView, Popover,
    PopoverMenu, SignalListItemFactory, SingleSelection, SortListModel, TreeExpander,
    TreeListModel, TreeListRow,
    cairo::Rectangle,
    gio::{self, FileInfo, Menu, MenuModel},
    glib::clone,
};
use sourceview5::prelude::*;

/// credit to https://codeberg.org/Cyncrovee/Datolite/src/branch/main/src/util.rs which i used as a reference
/// idk how the fuck to do this and the wiki doesn't really help
pub fn update(tree: &ListView, dir_path: &str) {
    // Scan for files
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

    let folder_popover = {
        let menu = Menu::new();
        let popover = PopoverMenu::builder()
            .menu_model(&menu)
            .has_arrow(false)
            .build();
        menu.append(Some("Change workspace"), Some("win.tab.new_scratch")); // TODO uh
        popover
    };

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
                // Context menus
                let tree_click = GestureClick::new();
                tree_click.set_button(3);
                tree_click.connect_pressed(clone!(
                    #[strong]
                    container,
                    #[strong]
                    folder_popover,
                    move |gesture, _, x, y| {
                        gesture.set_state(gtk::EventSequenceState::Claimed);
                        folder_popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                            x as i32, y as i32, 1, 1,
                        )));
                        folder_popover.unparent();
                        folder_popover.set_parent(&container);
                        // TODO set status in file tree somewhere that the window can also look,
                        // or use an action with passing data or whatever, or use a channel
                        folder_popover.popup();
                    }
                ));
                container.parent().unwrap().add_controller(tree_click);
            } else {
                label.set_text(&file_info.display_name());
                if let Some(icon) = file_info.symbolic_icon() {
                    image.set_from_gicon(&icon);
                } else {
                    image.set_icon_name(Some("text-x-generic-symbolic"));
                }
            }
            tree.set_list_row(Some(&row));
        }
    });

    // Hook things up
    tree.set_model(Some(&SingleSelection::new(Some(tree_model.clone()))));
    tree.set_factory(Some(&factory));
}
