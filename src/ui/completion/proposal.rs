use std::cell::RefCell;

use adw::{prelude::*, subclass::prelude::*};
use async_lsp::lsp_types::{
    CompletionItem, CompletionTextEdit, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};
use gtk::{glib, glib::Properties};
use sourceview5::{CompletionProposal, subclass::prelude::*};

mod imp {

    use super::*;
    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type=super::LspCompletionProposal)]
    pub struct LspCompletionProposal {
        #[property(get, set)]
        pub label: RefCell<Option<String>>,
        #[property(get, set)]
        pub comment: RefCell<Option<String>>,
        #[property(get, set)]
        pub documentation: RefCell<Option<String>>,
        #[property(get, set)]
        pub icon: RefCell<Option<String>>,

        pub item: RefCell<Option<CompletionItem>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LspCompletionProposal {
        const NAME: &'static str = "LspCompletionProposal";
        type Type = super::LspCompletionProposal;
        type Interfaces = (CompletionProposal,);
    }

    impl ObjectImpl for LspCompletionProposal {}

    impl CompletionProposalImpl for LspCompletionProposal {}
}

glib::wrapper! {
    pub struct LspCompletionProposal(ObjectSubclass<imp::LspCompletionProposal>)
        @implements sourceview5::CompletionProposal;
}

impl LspCompletionProposal {
    pub fn new() -> Self {
        glib::Object::builder::<Self>().build()
    }

    pub fn with_item(item: CompletionItem) -> Self {
        let proposal = glib::Object::builder::<Self>().build();
        proposal.imp().item.replace(Some(item.clone()));
        proposal.set_label(item.label.clone());
        proposal.set_comment(item.detail.unwrap_or(item.label));
        proposal.set_documentation(
            item.documentation
                .map(|d| {
                    match d {
                        Documentation::String(val) => val,
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::PlainText,
                            value: val,
                        }) => val,
                        // TODO Convert Markdown to Pango markup
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: val,
                        }) => val,
                    }
                })
                .unwrap_or_default(),
        );
        proposal.set_icon(if let Some(kind) = item.kind {
            format!("{:?}", kind).to_lowercase()
        } else {
            String::new()
        });
        proposal
    }

    pub fn apply(&self, buffer: sourceview5::Buffer) {
        let item = self.item().expect("Completion not fully formed");
        // TODO does it want me to replace the currently typed text to complete, or should I leave that, and the edit takes care of it??
        match item.text_edit {
            // Apply text edit with buffer.apply_text_edit
            Some(CompletionTextEdit::Edit(edit)) => {
                crate::ui::buffer::Buffer::apply_text_edit(&buffer, edit);
            }
            // Remove the replace range, then insert in the insert range.
            // Not supported by Kyanite because What the Actual Fuck is the docs trying to tell me to do. (see: client capabilities)
            Some(CompletionTextEdit::InsertAndReplace(_edit)) => (),
            // fallback: insert and replace with insert_text
            None => match item.insert_text_format {
                // NOTE GtkSourceView doesn't have support for tabstops afaik
                Some(InsertTextFormat::SNIPPET) => {
                    // Insert the text
                    buffer.insert_at_cursor(&item.insert_text.unwrap());
                    // TODO Select the first snippet tabstop
                    // TODO replace other tabstops with a name
                }
                _ => buffer.insert_at_cursor(&item.insert_text.unwrap()),
            },
        }
        // TODO: apply additional edits
    }

    pub fn item(&self) -> Option<CompletionItem> {
        self.imp().item.borrow().clone()
    }
}
