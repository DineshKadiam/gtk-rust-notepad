use std::path::Path;

use adw::{AlertDialog, ResponseAppearance, prelude::*};
use gtk::{Entry, glib::clone};

#[derive(Default, Debug)]
pub enum Kind {
    #[default]
    Rename,
    Create,
}

pub fn new(kind: Kind, path: String) -> AlertDialog {
    let dialog = AlertDialog::new(
        match kind {
            Kind::Rename => Some("Rename file..."),
            Kind::Create => Some("Create file..."),
        },
        None,
    );
    dialog.add_responses(&[
        ("cancel", "_Cancel"),
        (
            "apply",
            match kind {
                Kind::Rename => "_Rename",
                Kind::Create => "_Create",
            },
        ),
    ]);
    dialog.set_default_response(Some("cancel"));
    dialog.set_response_appearance("apply", ResponseAppearance::Suggested);
    dialog.set_response_enabled("apply", false);
    // Set input field
    let entry = Entry::new();
    entry.set_hexpand(true);
    dialog.set_extra_child(Some(&entry));
    entry.connect_changed(clone!(
        #[strong]
        dialog,
        move |entry| {
            dialog.set_response_enabled(
                "apply",
                !entry.text().trim().is_empty() && !entry.text().contains("/"),
            );
        }
    ));
    if let Kind::Rename = kind {
        let path_buf = Path::new(&path).to_path_buf();
        entry.set_text(path_buf.file_name().unwrap_or_default().to_str().unwrap());
    }
    // Set apply button
    dialog.connect_response(Some("apply"), move |dialog, _| {
        let binding = entry.text();
        let filename = binding.trim();
        let mut path_buf = Path::new(&path).to_path_buf();
        if !filename.is_empty() {
            match kind {
                Kind::Rename => {
                    std::fs::rename(
                        &path,
                        path_buf.parent().unwrap().join(filename).to_str().unwrap(),
                    )
                    .unwrap();
                }
                Kind::Create => {
                    if path_buf.is_file() {
                        path_buf = path_buf.parent().unwrap().to_path_buf();
                    }
                    std::fs::File::create(path_buf.join(filename).to_str().unwrap()).unwrap();
                }
            }
            adw::prelude::AdwDialogExt::close(dialog);
        }
    });
    dialog
}
