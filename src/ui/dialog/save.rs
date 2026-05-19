use adw::{AlertDialog, ResponseAppearance, prelude::*};
use gtk::{Align, CheckButton};

use crate::ui::buffer::Buffer;

/// Creates a dialog that prompts the user to save documents before closing a tab/window
pub fn new<C, D, S>(buffers: Vec<Buffer>, on_cancel: C, on_discard: D, on_save: S) -> AlertDialog
where
    C: Fn() + 'static,
    D: Fn() + 'static,
    S: Fn(&Buffer, bool) + 'static,
{
    // Create dialog
    let dialog = AlertDialog::new(
        Some("Save changes?"),
        Some(&format!(
            "Open documents contain unsaved changes. Changes which are not saved will be permanently lost.",
        )),
    );
    // Populate buttons
    dialog.add_responses(&[
        ("cancel", "_Cancel"),
        (
            "discard",
            if buffers.len() < 2 {
                "_Discard"
            } else if buffers.len() == 2 {
                "_Discard Both"
            } else {
                "_Discard All"
            },
        ),
        ("save", "_Save"),
    ]);
    dialog.set_response_appearance("discard", ResponseAppearance::Destructive);
    dialog.set_response_appearance("save", ResponseAppearance::Suggested);
    dialog.set_prefer_wide_layout(true);
    // Create/populate preferences group
    let group = adw::PreferencesGroup::new();
    buffers.iter().for_each(|buffer| {
        let row = adw::ActionRow::new();
        let check = CheckButton::new();
        check.set_active(true);
        check.set_valign(Align::Center);
        row.add_prefix(&check);
        row.set_activatable_widget(Some(&check));
        row.set_title(&buffer.clean_name(""));
        group.add(&row);
    });
    dialog.set_extra_child(Some(&group));
    // Connect responses
    dialog.connect_response(Some("cancel"), move |_, _| on_cancel());
    dialog.connect_response(Some("discard"), move |_, _| on_discard());
    dialog.connect_response(Some("save"), move |_, _| {
        let mut i = 0;
        for buffer in &buffers {
            let is_checked = group
                .row(i)
                .and_downcast_ref::<adw::ActionRow>()
                .unwrap()
                .first_child()
                .unwrap()
                .first_child()
                .unwrap()
                .first_child()
                .and_downcast_ref::<CheckButton>() // holy mother of god look at that chain
                .unwrap()
                .is_active();
            on_save(&buffer, is_checked); //.await
            i += 1;
        }
        // TODO wait for all dialogs to finish (or cancel), all saving I/O to finish, then do an on_save_Finish
        // TODO this will need an await or two
    });
    // yay
    dialog
}
