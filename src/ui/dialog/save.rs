use adw::{AlertDialog, ResponseAppearance, prelude::*};
use gtk::{
    Align, CheckButton, Label,
    glib::{self, clone},
};

use crate::{ui::buffer::Buffer, utils};

/// Creates a dialog that prompts the user to save documents before closing a tab/window
pub fn new<C, D, S>(
    buffers: Vec<Buffer>,
    on_cancel: C,
    on_close_finish: D,
    on_save: S,
) -> AlertDialog
where
    C: Fn() + 'static,
    D: Fn() + Clone + 'static,
    S: AsyncFn(&Buffer, bool) + Clone + 'static,
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
        row.set_title(&buffer.tab_name()); // TODO replace with clean name - get workspace uri
        if buffer.is_externally_modified() {
            row.set_subtitle("Externally modified");
            row.add_css_class("warning");
        }
        group.add(&row);
    });
    dialog.set_extra_child(Some(&group));
    // Connect responses
    dialog.connect_response(Some("cancel"), move |_, _| on_cancel());
    dialog.connect_response(
        Some("discard"),
        clone!(
            #[strong]
            on_close_finish,
            move |_, _| on_close_finish()
        ),
    );
    dialog.connect_response(Some("save"), move |_, _| {
        let group = group.clone();
        let buffers = buffers.clone();
        let on_save = on_save.clone();
        let on_close_finish = on_close_finish.clone();
        glib::spawn_future_local(async move {
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
                on_save(&buffer, is_checked).await;
                i += 1;
            }
            on_close_finish();
        });
    });
    // yay
    dialog
}
