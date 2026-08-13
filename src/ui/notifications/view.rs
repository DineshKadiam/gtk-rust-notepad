use adw::{prelude::*, subclass::prelude::*};
use glib::{Properties, clone};
use gtk::{CompositeTemplate, glib};

mod imp {

    use gtk::gio::{ActionEntry, SimpleActionGroup};

    use super::*;
    use crate::{
        notifications::{self, data::Event},
        ui::notifications::row::NotificationRow,
    };

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/notifs.blp")]
    #[properties(wrapper_type = super::NotificationsView)]
    pub struct NotificationsView {
        #[template_child]
        pub clear_button: TemplateChild<adw::ButtonRow>,
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

            // Clear action
            let action_group = SimpleActionGroup::new();
            action_group.add_action_entries([ActionEntry::builder("clear")
                .parameter_type(None)
                .activate(clone!(
                    #[strong(rename_to=notifs_list)]
                    self.notifs_list,
                    move |_, _, _| {
                        let mut _child = notifs_list.first_child();
                        let mut ids: Vec<String> = vec![];
                        while let Some(child) = _child {
                            if let Some(row) =
                                child.first_child().and_downcast_ref::<NotificationRow>()
                            {
                                ids.push(row.widget_name().into());
                            }
                            _child = child.next_sibling();
                        }
                        gtk::gio::spawn_blocking(move || {
                            ids.iter().for_each(|id| {
                                notifications::pop(id.to_string());
                            });
                        });
                    }
                ))
                .build()]);
            self.obj()
                .insert_action_group("notifs", Some(&action_group));

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
                                let progress = if let notifications::data::Status::Progress(p) =
                                    notif.status
                                {
                                    p as f64
                                } else {
                                    -1.0
                                };
                                remove(notifs_list.clone(), &mut labels, notif.id.clone());
                                let label = NotificationRow::new();
                                label.set_summary(notif.summary);
                                label.set_progress(progress);
                                label.set_widget_name(&notif.id);
                                if let notifications::data::Priority::High = notif.priority {
                                    button.add_css_class("error");
                                    label.add_css_class("error");
                                }
                                notifs_list.prepend(&label);
                                labels.push(label);
                            }
                            Event::Pop(id) => {
                                let row = remove(notifs_list.clone(), &mut labels, id);
                                if let Some(row) = row
                                    && row.has_css_class("error")
                                {
                                    button.remove_css_class("error");
                                }
                            }
                        }
                        button.set_visible(!labels.is_empty());
                        if button.is_visible() {
                            button.add_css_class("visible");
                        } else {
                            button.remove_css_class("visible");
                        }
                    }
                }
            ));
        }
    }
    impl WidgetImpl for NotificationsView {}
    impl BinImpl for NotificationsView {}

    fn remove(
        notifs_list: gtk::ListBox,
        labels: &mut Vec<NotificationRow>,
        id: String,
    ) -> Option<NotificationRow> {
        let mut _child = notifs_list.first_child();
        while let Some(child) = _child {
            if let Some(row) = child.first_child()
                && row.widget_name() == id
            {
                let row_type = row.clone().downcast::<NotificationRow>().ok();
                if let Some(p) = labels.iter().position(|l| l.widget_name() == id) {
                    labels.remove(p);
                }
                notifs_list.remove(&child);
                return row_type;
            }
            _child = child.next_sibling();
        }
        None
    }
}

glib::wrapper! {
pub struct NotificationsView(ObjectSubclass<imp::NotificationsView>)
    @extends adw::Bin, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable, gtk::Actionable;
}
