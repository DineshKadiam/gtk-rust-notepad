use adw::subclass::prelude::*;
use gtk::glib;
use sourceview5::CompletionProvider;
use sourceview5::subclass::prelude::*;

mod imp {

    use std::{cell::RefCell, sync::RwLock};

    use gtk::{
        MapListModel, StringList,
        gdk::Key,
        gio::{ListModel, ListStore},
        glib::translate::List,
    };
    use sourceview5::prelude::CompletionProposalExt;
    use vte4::{Cast, ObjectExt, TextBufferExt, WidgetExt};

    use crate::ui::completion::proposal::LspCompletionProposal;

    use super::*;
    #[derive(Debug, Default)]
    pub struct LspCompletionProvider {
        pub list: RefCell<Option<ListStore>>,
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
            println!("{:?}", proposal.label());
            context.buffer().unwrap().insert_at_cursor(
                &proposal
                    .label()
                    .unwrap()
                    .replace(&context.word().to_string(), ""),
            );
        }

        fn display(
            &self,
            context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
            cell: &sourceview5::CompletionCell,
        ) {
            let proposal = proposal.downcast_ref::<LspCompletionProposal>().unwrap();
            match cell.column() {
                sourceview5::CompletionColumn::Icon => cell.set_icon_name("neofox_floof"),
                sourceview5::CompletionColumn::TypedText => {
                    cell.set_text(Some(&proposal.label().unwrap()));
                }
                sourceview5::CompletionColumn::Comment => {
                    cell.set_text(Some(&proposal.comment().unwrap()));
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
            self.parent_refilter(context, model)
        }

        fn populate_future(
            &self,
            context: &sourceview5::CompletionContext,
        ) -> std::pin::Pin<Box<dyn Future<Output = Result<gtk::gio::ListModel, glib::Error>>>>
        {
            println!("populate_future");
            // println!("{:?}", context.word());
            let list = if let Some(list) = self.list.borrow().clone() {
                list
            } else {
                self.list
                    .replace(Some(ListStore::new::<LspCompletionProposal>()));
                self.list.borrow().clone().unwrap()
            };
            Box::pin(async move {
                list.remove_all();
                for i in 0..5 {
                    let p = LspCompletionProposal::new();
                    p.set_label(format!("Propsal test #{}", i).as_str());
                    p.set_comment(format!("Really fancy description #{}", i).as_str());
                    list.append(&p);
                }
                Ok(list.upcast::<ListModel>())
            })
        }

        fn title(&self) -> Option<glib::GString> {
            self.parent_title()
        }

        fn priority(&self, context: &sourceview5::CompletionContext) -> i32 {
            self.parent_priority(context)
        }

        fn is_trigger(&self, iter: &gtk::TextIter, c: char) -> bool {
            self.parent_is_trigger(iter, c)
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
