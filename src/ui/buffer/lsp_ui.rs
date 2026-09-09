use adw::Dialog;
use async_lsp::lsp_types::{
    AnnotatedTextEdit, CodeActionOrCommand, Hover, OneOf, TextDocumentEdit, TextEdit, WorkspaceEdit,
};

use super::*;
use crate::{position_iter, ui::dialog::hover::HoverDialog};

impl Buffer {
    /// Clear diagnostics
    pub fn clear_diagnostics(&self) {
        // Clear tags
        let buffer = self.get_sourceview_buffer();
        let bounds = buffer.bounds();
        {
            if buffer.tag_table().lookup("diag_error").is_some() {
                buffer.remove_tag_by_name("diag_error", &bounds.0, &bounds.1);
            }
            if buffer.tag_table().lookup("inlay").is_some() {
                buffer.remove_tag_by_name("inlay", &bounds.0, &bounds.1);
            }
        }
        // Clear annotations
        self.clear_annotations();
        // Clear source marks
        for i in 0..buffer.line_count() {
            buffer
                .source_marks_at_line(i, None)
                .iter()
                .for_each(|mark| {
                    if !mark.is_deleted() {
                        buffer.delete_mark(mark);
                    }
                });
        }
    }

    /// ???
    // TODO get rid of this and see if code actions works fine. lsp says i have to do this. i don't want to.
    pub fn store_diagnostics(&self, diagnostics: Vec<Diagnostic>) {
        self.imp().diagnostics.replace(diagnostics);
    }

    /// Highlight diaganostic
    pub fn highlight_diagnostic(
        &self,
        range: &Range,
        message: String,
        severity: DiagnosticSeverity,
    ) {
        let buffer = self.get_sourceview_buffer();

        let annotation = self
            .new_annotation(&message, range.end.line as i32)
            .unwrap();
        match severity {
            DiagnosticSeverity::ERROR => annotation.add_css_class("error"),
            DiagnosticSeverity::WARNING => annotation.add_css_class("warning"),
            _ => (),
        };

        // Add tag
        if let Some(start) = position_iter!(buffer, range.start)
            && let Some(end) = position_iter!(buffer, range.end)
        {
            if buffer.tag_table().lookup("diag_error").is_none() {
                buffer.create_tag(Some("diag_error"), &[("underline", &Underline::Error)]);
            }
            buffer.apply_tag_by_name("diag_error", &start, &end);
            // Create gutter mark
            if buffer
                .source_marks_at_line(range.start.line as i32, None)
                .is_empty()
            {
                buffer.create_source_mark(
                    None,
                    match severity {
                        DiagnosticSeverity::ERROR => "error",
                        DiagnosticSeverity::WARNING => "warn",
                        _ => "info",
                    },
                    &buffer.iter_at_line(range.start.line as i32).unwrap(),
                );
            }
        }
    }

    /// Shows code actions in an alert dialog
    pub fn show_code_actions(&self, actions: Vec<CodeActionOrCommand>) {
        if actions.is_empty() {
            self.imp()
                .toast_overlay
                .add_toast(adw::Toast::new("No code actions available"));
        } else {
            let dialog = Dialog::new();
            // Create list of actions
            let group = adw::PreferencesGroup::new();
            actions.iter().for_each(clone!(
                #[strong]
                dialog,
                #[strong]
                group,
                #[strong(rename_to=buffer)]
                self,
                move |action| {
                    match action {
                        CodeActionOrCommand::CodeAction(action) => {
                            let row = adw::ButtonRow::new();
                            let action = action.clone();
                            let dialog = dialog.clone();
                            let buffer = buffer.clone();
                            row.set_width_request(320);
                            row.set_title(&action.title);
                            row.connect_activated(move |_| {
                                if let Some(ref edit) = action.edit {
                                    let serialized = serde_json::to_string(&edit).unwrap();
                                    buffer
                                        .activate_action("win.lsp.edit", Some(&serialized.into()))
                                        .unwrap();
                                }
                                dialog.close();
                            });
                            group.add(&row);
                        }
                        _ => todo!(),
                    }
                }
            ));
            // Create dialog
            let container = gtk::Box::new(gtk::Orientation::Vertical, 16);
            let cancel_group = adw::PreferencesGroup::new();
            let cancel = adw::ButtonRow::new();
            container.set_margin_top(16);
            container.set_margin_bottom(16);
            container.set_margin_start(16);
            container.set_margin_end(16);
            container.append(&group);
            container.append(&cancel_group);
            cancel_group.add(&cancel);
            cancel.set_title("Cancel");
            cancel.add_css_class("destructive-action");
            cancel.connect_activated(clone!(
                #[strong]
                dialog,
                move |_| {
                    dialog.close();
                }
            ));
            dialog.set_title("Code actions…");
            dialog.set_child(Some(&container));
            dialog.present(Some(&self.clone()));
        }
    }

    /// Shows LSP hover resutls in an alert dialog
    pub fn show_lsp_hover(&self, actions: Hover) {
        // Get contents
        let contents = match actions.contents {
            async_lsp::lsp_types::HoverContents::Scalar(_marked_string) => todo!(),
            async_lsp::lsp_types::HoverContents::Array(_marked_strings) => todo!(),
            async_lsp::lsp_types::HoverContents::Markup(markup_content) => markup_content.value,
        };
        // Create dialog
        let dialog = HoverDialog::new(&contents);
        dialog.present(&self.clone());
    }

    /// Applies LSP workspace edits
    pub fn apply_edit(&self, _edit: WorkspaceEdit) {}

    /// Applies LSP document edits. Does not check for matching file paths.
    pub fn apply_document_edit(&self, edit: TextDocumentEdit) {
        let buffer = self.get_sourceview_buffer();
        for edit in edit.edits {
            let edit = match edit {
                OneOf::Left(text_edit) => text_edit,
                OneOf::Right(AnnotatedTextEdit {
                    text_edit,
                    annotation_id: _, // TODO, I don't understand this
                }) => text_edit,
            };
            Buffer::apply_text_edit(&buffer, edit);
        }
    }

    /// Applies LSP TextEdit edits
    pub fn apply_text_edit(buffer: &sourceview5::Buffer, edit: TextEdit) {
        // FIXME: panics if the start range isn't valid, which happens sometimes, for some reason
        if let Some(mut end) = position_iter!(buffer, edit.range.end)
            && edit.range.start != edit.range.end
        {
            let mut start = position_iter!(buffer, edit.range.start)
                .unwrap_or_else(|| panic!("Invalid position: {:?}", edit.range.start));
            buffer.delete(&mut start, &mut end);
        }
        let mut start = position_iter!(buffer, edit.range.start)
            .unwrap_or_else(|| panic!("Invalid position: {:?}", edit.range.start));
        buffer.insert(&mut start, &edit.new_text);
    }
}
