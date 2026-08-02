use adw::subclass::prelude::*;
use glib::Properties;
use gtk::{CompositeTemplate, glib};

mod imp {

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/welcome.blp")]
    #[properties(wrapper_type = super::WelcomeScreen)]
    pub struct WelcomeScreen {}

    #[glib::object_subclass]
    impl ObjectSubclass for WelcomeScreen {
        const NAME: &'static str = "WelcomeScreen";
        type Type = super::WelcomeScreen;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for WelcomeScreen {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for WelcomeScreen {}
    impl BinImpl for WelcomeScreen {}
}

glib::wrapper! {
pub struct WelcomeScreen(ObjectSubclass<imp::WelcomeScreen>)
    @extends adw::Bin, gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}
