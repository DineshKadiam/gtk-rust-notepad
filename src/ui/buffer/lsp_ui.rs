use async_lsp::lsp_types::CodeActionOrCommand;
use gtk::{Align, CheckButton};

use super::*;

impl Buffer {
    /// Clear diagnostics
    pub fn clear_diagnostics(&self) {
        self.imp().diagnostic_provider.borrow().remove_all();
        // Clear tags
        let buffer = self.get_sourceview_buffer();
        let bounds = buffer.bounds();
        {
            let mut exists = false;
            buffer.tag_table().foreach(|t| {
                if t.name().is_some_and(|n| n == "diag_error") {
                    exists = true;
                }
            });
            if exists {
                buffer.remove_tag_by_name("diag_error", &bounds.0, &bounds.1);
            }
        }
    }

    /// ???
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
        // Add annotation
        if create_annotation {
            self.imp()
                .diagnostic_provider
                .borrow()
                .add_annotation(&Annotation::new(
                    Some(&message.replace("\n", " / ")),
                    Some(
                        Icon::for_string(match severity {
                            DiagnosticSeverity::ERROR => "dialog-error-symbolic",
                            DiagnosticSeverity::WARNING => "dialog-warning-symbolic",
                            _ => "dialog-information-symbolic",
                        })
                        .unwrap(),
                    ),
                    range.start.line as i32,
                    match severity {
                        DiagnosticSeverity::ERROR => sourceview5::AnnotationStyle::Error,
                        DiagnosticSeverity::WARNING => sourceview5::AnnotationStyle::Warning,
                        _ => sourceview5::AnnotationStyle::Accent,
                    },
                ));
        }
        // Add tag
        let buffer = self.get_sourceview_buffer();
        if let Some(start) =
            buffer.iter_at_line_offset(range.start.line as i32, range.start.character as i32)
            && let Some(end) =
                buffer.iter_at_line_offset(range.end.line as i32, range.end.character as i32)
        {
            let mut exists = false;
            buffer.tag_table().foreach(|t| {
                if t.name().is_some_and(|n| n == "diag_error") {
                    exists = true;
                }
            });
            if !exists {
                buffer.create_tag(Some("diag_error"), &[("underline", &Underline::Error)]);
            }
            buffer.apply_tag_by_name("diag_error", &start, &end);
        }
    }

    /// Shows code actions in an alert dialog
    pub fn show_code_actions(&self, actions: Vec<CodeActionOrCommand>) {
        // Create list of actions
        let group = adw::PreferencesGroup::new();
        actions.iter().for_each(|action| {
            if let CodeActionOrCommand::CodeAction(action) = action {
                let row = adw::ButtonRow::new();
                row.set_width_request(320);
                row.set_title(&action.title);
                group.add(&row);
            }
        });
        // Create dialog
        let dialog = AlertDialog::new(Some("Code Actions"), None);
        dialog.add_responses(&[("cancel", "_Cancel")]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.set_extra_child(Some(&group));
        adw::prelude::AdwDialogExt::present(&dialog, Some(&self.clone()));
    }
}
