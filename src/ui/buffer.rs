use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib::{self};
use sourceview5::prelude::*;

mod imp {

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/tab.blp")]
    #[properties(wrapper_type = super::Buffer)]
    pub struct Buffer {
        #[template_child]
        pub source_view: TemplateChild<sourceview5::View>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Buffer {
        const NAME: &'static str = "Buffer";
        type Type = super::Buffer;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Buffer {}
    impl WidgetImpl for Buffer {}
    impl BoxImpl for Buffer {}
}

glib::wrapper! {
pub struct Buffer(ObjectSubclass<imp::Buffer>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Buffer {
    pub fn new() -> Self {
        glib::Object::builder::<Self>().build()
    }

    /// Creates a buffer ui/tab thingy with a SourceView Buffer object
    pub fn with_source(buf: sourceview5::Buffer) -> Self {
        let widget = glib::Object::builder::<Self>().build();
        widget.imp().source_view.set_buffer(Some(&buf));
        widget
    }
}
