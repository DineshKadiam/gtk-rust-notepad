use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;
use vte4::prelude::*;

mod imp {

    use std::cell::RefCell;

    use gtk::{Button, Label, ProgressBar, glib::clone};

    use crate::notifications;

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/notif_row.blp")]
    #[properties(wrapper_type = super::NotificationRow)]
    pub struct NotificationRow {
        #[template_child]
        pub label_summary: TemplateChild<Label>,
        #[template_child]
        pub progress_bar: TemplateChild<ProgressBar>,
        #[template_child]
        pub close_button: TemplateChild<Button>,
        #[property(get, set)]
        pub summary: RefCell<String>,
        #[property(get, set)]
        pub progress: RefCell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NotificationRow {
        const NAME: &'static str = "NotificationRow";
        type Type = super::NotificationRow;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for NotificationRow {
        fn constructed(&self) {
            self.parent_constructed();

            self.close_button.connect_activate(clone!(
                #[strong(rename_to=obj)]
                self.obj(),
                move |_| {
                    notifications::pop(obj.widget_name().into());
                }
            ));
        }
        fn dispatch_properties_changed(&self, p: &[glib::ParamSpec]) {
            for s in p {
                if s.name() == "summary" {
                    self.label_summary.set_text(&self.obj().summary());
                }
                if s.name() == "progress" {
                    self.progress_bar.set_visible(self.obj().progress() > 0.0);
                    self.progress_bar.set_fraction(self.obj().progress());
                }
            }
        }
    }
    impl WidgetImpl for NotificationRow {}
    impl BinImpl for NotificationRow {}
}

glib::wrapper! {
pub struct NotificationRow(ObjectSubclass<imp::NotificationRow>)
    @extends adw::Bin, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl NotificationRow {
    pub fn new() -> Self {
        glib::Object::builder::<Self>().build()
    }
}
