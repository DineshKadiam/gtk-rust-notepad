use std::path::Path;

use adw::{
    prelude::*, Dialog, EntryRow, HeaderBar, PreferencesGroup, PreferencesPage, ToolbarView,
};
use gtk::glib::clone;

use crate::notifications::{self, data::Notification};

#[derive(Default, Debug)]
pub enum Kind {
    #[default]
    Rename,
    NewFolder,
    NewFile,
}

pub fn new(kind: Kind, path: String) -> Dialog {
    let view = ToolbarView::builder().build();
    let dialog = Dialog::builder()
        .title(match kind {
            Kind::Rename => "Rename File",
            Kind::NewFolder => "New Folder",
            Kind::NewFile => "New File",
        })
        .child(&view)
        .content_width(450)
        .build();
    view.add_top_bar(&HeaderBar::new());

    // Set input field
    let entry = EntryRow::new();
    let group = PreferencesGroup::new();
    let page = PreferencesPage::new();
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(
            &gtk::Label::builder()
                .label("Name contains invalid character")
                .css_classes(["warning"])
                .build(),
        )
        .build();
    let button = gtk::Button::builder()
        .label(match kind {
            Kind::Rename => "Rename",
            _ => "Create",
        })
        .halign(gtk::Align::Center)
        .sensitive(false)
        .css_classes(["pill", "suggested-action"])
        .build();
    page.add(&group);
    group.add(&entry);
    group.add(&revealer);
    group.add(&button);
    entry.set_title("File Name");
    view.set_content(Some(&page));
    entry.connect_changed(clone!(
        #[strong]
        button,
        move |entry| {
            button.set_sensitive(!entry.text().trim().is_empty() && !entry.text().contains("/"));
        }
    ));
    if let Kind::Rename = kind {
        let path_buf = Path::new(&path).to_path_buf();
        entry.set_text(path_buf.file_name().unwrap_or_default().to_str().unwrap());
    }
    // Set apply button
    button.connect_clicked(clone!(
        #[strong]
        dialog,
        move |_| {
            let binding = entry.text();
            let filename = binding.trim();
            let mut path_buf = Path::new(&path).to_path_buf();
            if !filename.is_empty() {
                let err = match kind {
                    Kind::Rename => std::fs::rename(
                        &path,
                        path_buf.parent().unwrap().join(filename).to_str().unwrap(),
                    )
                    .is_err(),
                    Kind::NewFile => {
                        if path_buf.is_file() {
                            path_buf = path_buf.parent().unwrap().to_path_buf();
                        }
                        std::fs::File::create(path_buf.join(filename).to_str().unwrap()).is_err()
                    }
                    Kind::NewFolder => {
                        if path_buf.is_file() {
                            path_buf = path_buf.parent().unwrap().to_path_buf();
                        }
                        std::fs::create_dir(path_buf.join(filename).to_str().unwrap()).is_err()
                    }
                };
                if err {
                    notifications::push(Notification {
                        status: notifications::data::Status::Normal,
                        priority: notifications::data::Priority::High,
                        summary: format!("File operation failed for {filename}"),
                        details: String::new(),
                        id: "fops_failed".to_string(),
                    });
                }
                adw::prelude::AdwDialogExt::close(&dialog);
            }
        }
    ));
    dialog
}
