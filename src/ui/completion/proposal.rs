use adw::prelude::*;
use adw::subclass::prelude::*;
use async_lsp::lsp_types::CompletionItem;
use gtk::glib;
use gtk::glib::Properties;
use sourceview5::CompletionProposal;
use sourceview5::subclass::prelude::*;
use std::cell::RefCell;

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
        proposal
    }

    pub fn item(&self) -> Option<CompletionItem> {
        self.imp().item.borrow().clone()
    }
}
