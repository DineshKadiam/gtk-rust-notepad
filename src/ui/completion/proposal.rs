use std::cell::RefCell;

use adw::{prelude::*, subclass::prelude::*};
use async_lsp::lsp_types::{
    CompletionItem, CompletionTextEdit, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};
use gtk::{
    TextSearchFlags,
    glib::{self, Properties},
};
use sourceview5::{CompletionProposal, subclass::prelude::*};

use crate::ui::buffer::Buffer;

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

    #[glib::derived_properties]
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
        let initial_cursor = buffer.cursor_position();
        let mut insert_text = item.insert_text.unwrap_or(item.label);
        buffer.begin_user_action();
        // TODO does it want me to replace the currently typed text to complete, or should I leave that, and the edit takes care of it??
        match item.text_edit {
            // Apply text edit with buffer.apply_text_edit
            Some(CompletionTextEdit::Edit(edit)) => {
                insert_text = edit.new_text.clone();
                crate::ui::buffer::Buffer::apply_text_edit(&buffer, edit);
            }
            // Remove the replace range, then insert in the insert range.
            // Not supported by Kyanite because What the Actual Fuck is the docs trying to tell me to do. (see: client capabilities)
            Some(CompletionTextEdit::InsertAndReplace(_edit)) => (),
            // fallback: insert and replace with insert_text
            None => buffer.insert_at_cursor(&insert_text),
        }
        // Handle snippets
        if let Some(InsertTextFormat::SNIPPET) = item.insert_text_format {
            println!("[info] Snippet inserted: `{0}`", insert_text);
            // TODO fully support tabstop syntax. with all its quirks (what have I gotten myself into)
            // TODO https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#snippet_syntax
            // Replace placeholders
            while let Some((mut tabstop_start, _)) =
                buffer.iter_at_offset(initial_cursor).forward_search(
                    "${",
                    TextSearchFlags::empty(),
                    Some(&buffer.iter_at_offset(buffer.cursor_position())),
                )
                && let Some((_, mut tabstop_end)) = tabstop_start.forward_search(
                    "}",
                    TextSearchFlags::empty(),
                    Some(&buffer.iter_at_offset(buffer.cursor_position())),
                )
            {
                let text = buffer.text(&tabstop_start, &tabstop_end, false);
                buffer.delete(&mut tabstop_start, &mut tabstop_end);
                buffer.insert(&mut tabstop_start, &text[4..(text.len() - 1)]);
            }
            // finish -- Place cursor at $0
            let start = buffer.iter_at_offset(initial_cursor);
            let end = buffer.iter_at_offset(buffer.cursor_position());
            if let Some((mut start, mut end)) =
                start.forward_search("$0", TextSearchFlags::empty(), Some(&end))
            {
                buffer.delete(&mut start, &mut end);
                buffer.place_cursor(&start);
            }
        }
        // Apply additional edits
        if let Some(edits) = item.additional_text_edits {
            for edit in edits {
                Buffer::apply_text_edit(&buffer, edit);
            }
        }
        buffer.end_user_action();
    }

    pub fn item(&self) -> Option<CompletionItem> {
        self.imp().item.borrow().clone()
    }
}
