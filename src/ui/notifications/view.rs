use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use gtk::CompositeTemplate;
use gtk::glib;

mod imp {

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
        #[template_child]
        pub button: TemplateChild<gtk::MenuButton>,
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

            self.button.set_sensitive(false);
            self.button.connect_active_notify(move |button| {
                button.set_always_show_arrow(false);
            });

            glib::spawn_future_local(clone!(
                #[strong(rename_to=notifs_list)]
                self.notifs_list,
                #[strong(rename_to=button)]
                self.button,
                async move {
                    let rx = notifications::subscribe();
                    let mut labels: Vec<NotificationRow> = vec![];
                    while let Ok(event) = rx.recv().await {
                        match event {
                            Event::Push(notif) => {
                                button.set_always_show_arrow(true);
                                button.set_sensitive(!labels.is_empty());
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
                            Event::Pop(id) => {
                                let mut _child = notifs_list.first_child();
                                while let Some(child) = _child {
                                    if let Some(row) = child.first_child()
                                        && row.widget_name() == id
                                    {
                                        labels.remove(
                                            labels
                                                .iter()
                                                .position(|l| l.widget_name() == id)
                                                .unwrap(),
                                        );
                                        notifs_list.remove(&child);
                                        break;
                                    }
                                    _child = child.next_sibling();
                                }
                                button.set_sensitive(!labels.is_empty());
                            }
                        }
                    }
                }
            ));
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
