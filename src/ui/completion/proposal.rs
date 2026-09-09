use adw::subclass::prelude::*;
use async_lsp::lsp_types::CompletionItem;
use gtk::glib;
use sourceview5::subclass::prelude::*;

mod imp {

    use std::sync::RwLock;

    use async_lsp::lsp_types::CompletionItem;
    use gtk::glib::Properties;
    use sourceview5::CompletionProposal;

    use super::*;
    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type=super::LspCompletionProposal)]
    pub struct LspCompletionProposal {
        pub label: RwLock<Option<String>>,
        pub comment: RwLock<Option<String>>,
        pub documentation: RwLock<Option<String>>,
        pub item: RwLock<Option<CompletionItem>>,
        pub icon: RwLock<Option<String>>,
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

    pub fn set_label(&self, label: &str) {
        *self.imp().label.write().unwrap() = Some(label.to_string())
    }

    pub fn set_comment(&self, label: &str) {
        *self.imp().comment.write().unwrap() = Some(label.to_string())
    }

    pub fn set_documentation(&self, label: &str) {
        *self.imp().documentation.write().unwrap() = Some(label.to_string())
    }

    pub fn set_item(&self, item: Option<CompletionItem>) {
        *self.imp().item.write().unwrap() = item;
    }

    pub fn set_icon(&self, icon: &str) {
        *self.imp().icon.write().unwrap() = Some(icon.to_string());
    }

    pub fn label(&self) -> Option<String> {
        (*self.imp().label.read().unwrap()).clone()
    }

    pub fn comment(&self) -> Option<String> {
        (*self.imp().comment.read().unwrap()).clone()
    }

    pub fn documentation(&self) -> Option<String> {
        (*self.imp().documentation.read().unwrap()).clone()
    }

    pub fn item(&self) -> Option<CompletionItem> {
        (*self.imp().item.read().unwrap()).clone()
    }

    pub fn icon(&self) -> Option<String> {
        (*self.imp().icon.read().unwrap()).clone()
    }
}
