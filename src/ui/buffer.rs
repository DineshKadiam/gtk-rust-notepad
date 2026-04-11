use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;
use sourceview5::prelude::*;
use std::cell::RefCell;

mod imp {

    use gtk::{EventControllerMotion, Revealer, glib::clone};

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/tab.blp")]
    #[properties(wrapper_type = super::Buffer)]
    pub struct Buffer {
        #[template_child]
        pub source_view: TemplateChild<sourceview5::View>,
        #[template_child]
        pub source_map: TemplateChild<sourceview5::Map>,
        #[template_child]
        pub map_revealer: TemplateChild<Revealer>,
        #[property(get, set)]
        pub file_path: RefCell<String>,
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
    impl ObjectImpl for Buffer {
        fn constructed(&self) {
            self.parent_constructed();
            // Show map on mouse motion only
            let motion_controller = EventControllerMotion::new();
            motion_controller.connect_enter(clone!(
                #[strong(rename_to=revealer)]
                self.map_revealer,
                move |_controller, _, _| {
                    revealer.set_reveal_child(true);
                }
            ));
            motion_controller.connect_leave(clone!(
                #[strong(rename_to=revealer)]
                self.map_revealer,
                move |_controller| {
                    revealer.set_reveal_child(false);
                }
            ));
            self.obj().add_controller(motion_controller);
        }
    }
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
