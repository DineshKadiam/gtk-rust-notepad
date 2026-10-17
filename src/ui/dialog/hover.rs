use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::{glib, Align, CompositeTemplate, Orientation};

use crate::gmark_config;

mod imp {

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/hover.blp")]
    #[properties(wrapper_type = super::HoverDialog)]
    pub struct HoverDialog {
        #[template_child]
        pub bin: TemplateChild<adw::Bin>,
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

        dialog
            .imp()
            .bin
            .set_child(Some(&gmark::parse(text, gmark_config!())));
        dialog
    }

    pub fn new_multiple(text: &[String]) -> Self {
        let dialog = glib::Object::builder::<Self>().build();
        let contents = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(16)
            .build();
        contents.set_valign(Align::Start);
        for msg in text {
            let out = gmark::parse(msg, gmark_config!());
            out.set_css_classes(&["card", "padded"]);
            out.queue_resize();
            contents.append(&out);
        }
        dialog.imp().bin.set_child(Some(&contents));
        dialog
    }

    pub fn present(&self, parent: &impl IsA<gtk::Widget>) {
        AdwDialogExt::present(self, Some(parent));
        // Hack to make sure children are properly sized, Because Gtk
        let bin = self.imp().bin.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            bin.queue_resize();
            glib::ControlFlow::Break
        });
    }
}
