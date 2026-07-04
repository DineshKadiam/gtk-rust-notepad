use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::gdk::RGBA;
use gtk::gio::Cancellable;
use gtk::glib;
use gtk::glib::SpawnFlags;
use vte4::prelude::*;

mod imp {

    use gtk::{Label, glib::clone};

    use crate::{
        notifications::{self, data::Event},
        ui::notifications::row::NotificationRow,
    };

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/notifs.blp")]
    #[properties(wrapper_type = super::NotificationsView)]
    pub struct NotificationsView {
        #[template_child]
        pub notifs_list: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NotificationsView {
        const NAME: &'static str = "NotificationsView";
        type Type = super::NotificationsView;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for NotificationsView {
        fn constructed(&self) {
            self.parent_constructed();

            glib::spawn_future_local(clone!(
                #[strong(rename_to=notifs_list)]
                self.notifs_list,
                async move {
                    let rx = notifications::subscribe();
                    let mut labels: Vec<NotificationRow> = vec![];
                    while let Ok(event) = rx.recv().await {
                        match event {
                            Event::Push(notif) => {
                                let progress = if let notifications::data::Status::Progress(p) =
                                    notif.status
                                {
                                    p as f64
                                } else {
                                    -1.0
                                };
                                if let Some(label) =
                                    labels.iter().find(|l| l.widget_name() == notif.id)
                                    && !notif.id.is_empty()
                                {
                                    label.set_summary(notif.summary);
                                    label.set_progress(progress);
                                } else {
                                    let label = NotificationRow::new();
                                    label.set_summary(notif.summary);
                                    label.set_progress(progress);
                                    label.set_widget_name(&notif.id);
                                    notifs_list.prepend(&label);
                                    labels.push(label);
                                }
                            }
                            _ => (),
                        }
                    }
                }
            ));
            /*
            for _ in 0..10 {
                notifications::push(notifications::data::Notification {
                    summary: "test!".to_string(),
                    status: notifications::data::Status::Progress(0.5),
                    ..Default::default()
                });
            }*/
        }
    }
    impl WidgetImpl for NotificationsView {}
    impl BinImpl for NotificationsView {}
}

glib::wrapper! {
pub struct NotificationsView(ObjectSubclass<imp::NotificationsView>)
    @extends adw::Bin, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}
