use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::{CompositeTemplate, Label, glib};

mod imp {

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/hover.blp")]
    #[properties(wrapper_type = super::HoverDialog)]
    pub struct HoverDialog {
        #[template_child]
        pub label: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HoverDialog {
        const NAME: &'static str = "HoverDialog";
        type Type = super::HoverDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for HoverDialog {}
    impl WidgetImpl for HoverDialog {}
    impl WindowImpl for HoverDialog {}
    impl DialogImpl for HoverDialog {}
    impl AdwDialogImpl for HoverDialog {}
}

glib::wrapper! {
pub struct HoverDialog(ObjectSubclass<imp::HoverDialog>)
    @extends adw::Dialog, gtk::Dialog, gtk::Window, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl HoverDialog {
    pub fn new(text: &str) -> Self {
        let dialog = glib::Object::builder::<Self>().build();
        dialog.imp().label.set_text(text);
        dialog
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        AdwDialogExt::present(self, Some(parent));
    }
}
