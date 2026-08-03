// Actions

use async_lsp::lsp_types::Position;
use gtk::gio::{ActionEntry, SimpleActionGroup};

use super::*;

impl Buffer {
    pub fn setup_actions(&self) {
        let group = SimpleActionGroup::new();
        group.add_action_entries([ActionEntry::builder("lsp.actions")
            .parameter_type(None)
            .activate(clone!(
                #[strong(rename_to=buffer)]
                self,
                move |_, _, _| {
                    if let Some(path) = buffer.file_path()
                        && let Some(provider) = buffer.imp().completion_provider.borrow().clone()
                        && let Some(workspace_path) = provider.imp().workspace_uri.borrow().clone()
                    {
                        let s_buffer = buffer.get_sourceview_buffer();
                        let iter = s_buffer.selection_bounds().unwrap();
                        let diags = buffer.imp().diagnostics.borrow().clone();
                        let start = Position {
                            line: iter.0.line() as u32,
                            character: iter.0.line_offset() as u32,
                        };
                        let end = Position {
                            line: iter.1.line() as u32,
                            character: iter.1.line_offset() as u32,
                        };
                        println!("{:?}, {:?}", start, end);
                        glib::spawn_future(async move {
                            if let Some(actions) =
                                lsp::request::actions(&path, &workspace_path, start, end, diags)
                                    .await
                            {
                                dbg!(actions);
                            } else {
                                println!("actions are a None");
                            }
                        });
                    }
                }
            ))
            .build()]);
        self.insert_action_group("buffer", Some(&group));
    }
}
