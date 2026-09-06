// Actions

use async_lsp::lsp_types::Position;
use gtk::gio::{ActionEntry, SimpleActionGroup};

use super::*;
use crate::iter_position;

impl Buffer {
    pub fn setup_actions(&self) {
        let group = SimpleActionGroup::new();
        group.add_action_entries([
            ActionEntry::builder("lsp.actions")
                .parameter_type(None)
                .activate(clone!(
                    #[strong(rename_to=buffer)]
                    self,
                    move |_, _, _| {
                        if let Some(path) = buffer.file_path() {
                            let workspace_path = buffer.imp().workspace_uri.borrow().clone();
                            let s_buffer = buffer.get_sourceview_buffer();
                            let iter = s_buffer.selection_bounds().unwrap_or_else(|| {
                                (
                                    s_buffer.iter_at_offset(s_buffer.cursor_position()),
                                    s_buffer.iter_at_offset(s_buffer.cursor_position() + 1),
                                )
                            });
                            let start = iter_position!(iter.0);
                            let end = iter_position!(iter.1);
                            glib::spawn_future_local(clone!(
                                #[strong]
                                buffer,
                                async move {
                                    lsp::loading::add(1);
                                    if let Some(actions) = lsp::request::actions(
                                        &path,
                                        &workspace_path,
                                        start,
                                        end,
                                        buffer.imp().diagnostics.borrow().clone(),
                                    )
                                    .await
                                    {
                                        buffer.show_code_actions(actions);
                                    }
                                    lsp::loading::sub(1);
                                }
                            ));
                        }
                    }
                ))
                .build(),
            ActionEntry::builder("lsp.hover")
                .parameter_type(None)
                .activate(clone!(
                    #[strong(rename_to=buffer)]
                    self,
                    move |_, _, _| {
                        if let Some(path) = buffer.file_path() {
                            let workspace_path = buffer.imp().workspace_uri.borrow().clone();
                            let s_buffer = buffer.get_sourceview_buffer();
                            let position =
                                iter_position!(s_buffer.iter_at_offset(s_buffer.cursor_position()));
                            glib::spawn_future_local(clone!(
                                #[strong]
                                buffer,
                                async move {
                                    lsp::loading::add(1);
                                    if let Some(hover) =
                                        lsp::request::hover(&path, &workspace_path, position).await
                                    {
                                        buffer.show_lsp_hover(hover);
                                    }
                                    lsp::loading::sub(1);
                                }
                            ));
                        }
                    }
                ))
                .build(),
        ]);
        self.insert_action_group("buffer", Some(&group));
    }
}
