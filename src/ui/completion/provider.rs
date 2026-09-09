use std::cell::RefCell;

use adw::subclass::prelude::*;
use gtk::{
    ClosureExpression, Expression, FilterListModel, StringFilter,
    gio::{ListModel, ListStore},
    glib,
    glib::closure,
    prelude::*,
};
use sourceview5::{CompletionProvider, subclass::prelude::*};

use crate::ui::completion::proposal::LspCompletionProposal;

mod imp {

    use async_lsp::lsp_types::{
        CompletionResponse, CompletionTextEdit, Documentation, MarkupContent, MarkupKind, Position,
        TextEdit,
    };
    use gtk::{
        Align,
        pango::{AttrFontDesc, AttrList, FontDescription},
    };

    use super::*;
    use crate::{iter_position, lsp};

    #[derive(Debug, Default)]
    pub struct LspCompletionProvider {
        pub uri: RefCell<Option<String>>,
        pub workspace_uri: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LspCompletionProvider {
        const NAME: &'static str = "LspCompletionProvider";
        type Type = super::LspCompletionProvider;
        type Interfaces = (CompletionProvider,);
    }

    impl ObjectImpl for LspCompletionProvider {}

    impl CompletionProviderImpl for LspCompletionProvider {
        fn activate(
            &self,
            context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
        ) {
            let proposal = proposal.downcast_ref::<LspCompletionProposal>().unwrap();
            let buffer = context.buffer().unwrap();
            let mut cursor_iter = buffer.bounds().0;
            // Delete typed words
            cursor_iter.forward_chars(buffer.cursor_position());
            for _ in 0..(context.word().len()) {
                buffer.backspace(&mut cursor_iter, false, true);
            }
            // Insert
            buffer.insert_at_cursor(&proposal.insert_text().unwrap_or(proposal.label().unwrap()));
        }

        fn display(
            &self,
            _: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
            cell: &sourceview5::CompletionCell,
        ) {
            let proposal = proposal.downcast_ref::<LspCompletionProposal>().unwrap();
            // TODO: constant move it out
            let icon_attrs = AttrList::new();
            icon_attrs.insert(AttrFontDesc::new(&FontDescription::from_string(
                "Monospace Bold Italic",
            )));
            match cell.column() {
                sourceview5::CompletionColumn::After => {
                    cell.set_text_with_attributes(&proposal.icon().unwrap(), &icon_attrs);
                }
                sourceview5::CompletionColumn::TypedText => {
                    cell.set_text(Some(&proposal.label().unwrap()));
                }
                sourceview5::CompletionColumn::Comment => {
                    cell.set_text(Some(&proposal.comment().unwrap()))
                }
                sourceview5::CompletionColumn::Details => {
                    let scroll_window = gtk::ScrolledWindow::new();
                    let label = gtk::Label::new(None);
                    label.set_markup(&proposal.documentation().unwrap());
                    scroll_window.set_child(Some(&label));
                    cell.set_hexpand(true);
                    cell.set_vexpand(true);
                    cell.set_halign(Align::Fill);
                    cell.set_valign(Align::Start);
                    cell.set_height_request(256);
                    cell.set_widget(&scroll_window);
                }
                _ => (),
            }
        }

        fn key_activates(
            &self,
            context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
            keyval: gtk::gdk::Key,
            state: gtk::gdk::ModifierType,
        ) -> bool {
            // TODO: KEYBINDS
            self.parent_key_activates(context, proposal, keyval, state)
        }

        fn refilter(&self, context: &sourceview5::CompletionContext, model: &gtk::gio::ListModel) {
            let model = model.downcast_ref::<FilterListModel>().unwrap();
            model
                .filter()
                .and_downcast_ref::<gtk::StringFilter>()
                .unwrap()
                .set_search(Some(context.word().as_ref()));
        }

        fn populate_future(
            &self,
            context: &sourceview5::CompletionContext,
        ) -> std::pin::Pin<Box<dyn Future<Output = Result<gtk::gio::ListModel, glib::Error>>>>
        {
            let uri = self.uri.borrow().clone();
            let workspace_uri = self.workspace_uri.borrow().clone();
            let buffer = context.buffer().unwrap();
            let iter = buffer.iter_at_offset(buffer.cursor_position());
            Box::pin(async move {
                // Create list of proposals
                let list = ListStore::new::<LspCompletionProposal>();
                if let Some(uri) = uri
                    && let Some(workspace_uri) = workspace_uri
                    && let Some(response) =
                        lsp::request::completion(&uri, &workspace_uri, iter_position!(iter)).await
                {
                    let items = match response {
                        CompletionResponse::List(list) => list.items,
                        CompletionResponse::Array(array) => array
                    };
                    for item in items {
                        let p = LspCompletionProposal::new();
                        p.set_label(&item.label);
                        p.set_item(Some(item.clone()));
                        p.set_comment(&item.detail.unwrap_or(item.label));
                        p.set_documentation(
                            &item
                                .documentation
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
                        p.set_icon(&if let Some(kind) = item.kind {
                            format!("{:?}", kind).to_lowercase()
                        } else {
                            String::new()
                        });
                        list.append(&p);
                    }
                }
                // Search list
                // TODO expression can be brought out
                let expression = ClosureExpression::new::<String>(
                    Expression::NONE,
                    closure!(|object: glib::Object| {
                        if let Some(proposal) = object.downcast_ref::<LspCompletionProposal>() {
                            Some(proposal.label().unwrap_or_default())
                        } else {
                            Some(String::new())
                        }
                    }),
                );

                let filter = StringFilter::new(Some(expression));
                filter.set_match_mode(gtk::StringFilterMatchMode::Prefix);
                let model = FilterListModel::new(Some(list), Some(filter));
                Ok(model.upcast::<ListModel>())
            })
        }

        fn is_trigger(&self, iter: &gtk::TextIter, c: char) -> bool {
            self.parent_is_trigger(iter, c) || c == '.'
        }

        fn list_alternates(
            &self,
            context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
        ) -> Vec<sourceview5::CompletionProposal> {
            self.parent_list_alternates(context, proposal)
        }
    }
}

glib::wrapper! {
    pub struct LspCompletionProvider(ObjectSubclass<imp::LspCompletionProvider>)
        @implements sourceview5::CompletionProvider;
}

impl LspCompletionProvider {
    pub fn new() -> Self {
        glib::Object::builder::<Self>().build()
    }
}












