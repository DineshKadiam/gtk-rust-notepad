use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use sourceview5::CompletionProposal;
use sourceview5::subclass::prelude::*;

mod imp {

    use std::sync::RwLock;

    use gtk::glib::Properties;

    use super::*;
    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type=super::LspCompletionProposal)]
    pub struct LspCompletionProposal {
        pub label: RwLock<Option<String>>,
        pub comment: RwLock<Option<String>>,
        pub insert_text: RwLock<Option<String>>,
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

    pub fn set_insert_text(&self, insert_text: Option<String>) {
        *self.imp().insert_text.write().unwrap() = insert_text;
    }

    pub fn set_icon(&self, icon: &str) {
        *self.imp().icon.write().unwrap() = Some(icon.to_string());
    }

    pub fn label(&self) -> Option<String> {
        if let Some(ref string) = *self.imp().label.read().unwrap() {
            Some(string.clone())
        } else {
            None
        }
    }

    pub fn comment(&self) -> Option<String> {
        if let Some(ref string) = *self.imp().comment.read().unwrap() {
            Some(string.clone())
        } else {
            None
        }
    }

    pub fn insert_text(&self) -> Option<String> {
        if let Some(ref string) = *self.imp().insert_text.read().unwrap() {
            Some(string.clone())
        } else {
            None
        }
    }

    pub fn icon(&self) -> Option<String> {
        if let Some(ref string) = *self.imp().icon.read().unwrap() {
            Some(string.clone())
        } else {
            None
        }
    }
}
