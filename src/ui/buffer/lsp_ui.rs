use adw::Dialog;
use async_lsp::lsp_types::{
    AnnotatedTextEdit, CodeActionOrCommand, DocumentChanges, Hover, OneOf, WorkspaceEdit,
};
use sourceview5::{GutterRendererPixbuf, GutterRendererText, MarkAttributes};

use super::*;
use crate::{position_iter, ui::dialog::hover::HoverDialog};

impl Buffer {
    /// Clear diagnostics
    pub fn clear_diagnostics(&self) {
        self.imp().diagnostic_provider.borrow().remove_all();
        // Clear tags
        let buffer = self.get_sourceview_buffer();
        let bounds = buffer.bounds();
        {
            if  buffer.tag_table().lookup("diag_error").is_some() {
                buffer.remove_tag_by_name("diag_error", &bounds.0, &bounds.1);
            }
        }
        // Clear source marks
        for i in 0..buffer.line_count() {
            buffer.source_marks_at_line(i, None).iter().for_each(|mark| {
                buffer.delete_mark(mark);
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
        create_annotation: bool,
    ) {
        let buffer = self.get_sourceview_buffer();
        // Add annotation
        if create_annotation {
            let annotation = Annotation::new(
                    Some(&(message.replace("\n", "  ") + "\t\t")),
                    None::<Icon>,
                    range.start.line as i32,
                    match severity {
                        DiagnosticSeverity::ERROR => sourceview5::AnnotationStyle::Error,
                        DiagnosticSeverity::WARNING => sourceview5::AnnotationStyle::Warning,
                        _ => sourceview5::AnnotationStyle::Accent,
                    },
                );
            self.imp()
                .diagnostic_provider
                .borrow()
                .add_annotation(&annotation);
        }
        // Add tag
        if let Some(start) = position_iter!(buffer, range.start)
            && let Some(end) = position_iter!(buffer, range.end)
        {
            if buffer.tag_table().lookup("diag_error").is_none() {
                buffer.create_tag(Some("diag_error"), &[("underline", &Underline::Error)]);
            }
            buffer.apply_tag_by_name("diag_error", &start, &end);
            // Create gutter mark
            if buffer.source_marks_at_line(range.start.line as i32, None).is_empty() {
                buffer.create_source_mark(
                    None,
                    match severity {
                        DiagnosticSeverity::ERROR => "error",
                        DiagnosticSeverity::WARNING => "warn",
                        _ => "info",
                    },
                    &buffer.iter_at_line(range.start.line as i32).unwrap()
                );
            }
        }
    }

    /// Shows code actions in an alert dialog
    pub fn show_code_actions(&self, actions: Vec<CodeActionOrCommand>) {
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
                // TODO handle command
                if let CodeActionOrCommand::CodeAction(action) = action {
                    let row = adw::ButtonRow::new();
                    let action = action.clone();
                    let dialog = dialog.clone();
                    let buffer = buffer.clone();
                    row.set_width_request(320);
                    row.set_title(&action.title);
                    row.connect_activated(move |_| {
                        if let Some(ref edit) = action.edit {
                            buffer.apply_edit(edit.clone());
                        }
                        dialog.close();
                    });
                    group.add(&row);
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

    /// Applies LSP workspace edits
    pub fn apply_edit(&self, edit: WorkspaceEdit) {
        // TODO: "But Spike, aren't these *workspace* edits?" Yeah..............
        // TODO: Having this globally handled by e.g. a window would be much preferred, but
        // TODO: right now this is just a proof-of-concept.
        // TODO: Handle DocumentChanges::Operations
        let buffer = self.get_sourceview_buffer();
        if let Some(DocumentChanges::Edits(edits)) = edit.document_changes {
            for edit in edits {
                let path = edit.text_document.uri.path();
                if path == self.file_path().unwrap() {
                    for edit in edit.edits {
                        let edit = match edit {
                            OneOf::Left(text_edit) => text_edit,
                            OneOf::Right(AnnotatedTextEdit {
                                text_edit,
                                annotation_id: _,
                            }) => text_edit,
                        };
                        // FIXME: panics if the range isn't valid, which happens sometimes, for some reason
                        let mut start = position_iter!(buffer, edit.range.start).unwrap();
                        let mut end = position_iter!(buffer, edit.range.end).unwrap();
                        buffer.delete(&mut start, &mut end);
                        start = position_iter!(buffer, edit.range.start).unwrap();
                        buffer.insert(&mut start, &edit.new_text);
                    }
                } else {
                    todo!();
                    // "[fatal] WorkspaceEdit external file paths not supported (see src/ui/buffer/lsp_ui.rs::apply_edit)"
                }
            }
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
}



















































































































































































