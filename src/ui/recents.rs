use crate::config::Config;
use crate::utils;
use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;
use gtk::glib::clone;

mod imp {

    use gtk::ListBox;

    use crate::ui::sidebar;

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/recents.blp")]
    #[properties(wrapper_type = super::RecentsDialog)]
    pub struct RecentsDialog {
        #[template_child]
        pub files: TemplateChild<ListBox>,
        #[template_child]
        pub folders: TemplateChild<ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RecentsDialog {
        const NAME: &'static str = "RecentsDialog";
        type Type = super::RecentsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for RecentsDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let recents = gtk::RecentManager::default();
            let files = self.files.clone();
            let folders = self.folders.clone();
            let this = self.obj().clone();
            let mut items = recents.items().into_iter().collect::<Vec<gtk::RecentInfo>>();
            items.sort_by(|a, b| {
            a.age().cmp(&b.age())});
            items.iter().for_each(|item| {
                if item.exists() && item.is_local() {
                    let row = adw::ActionRow::new();
                    let button = gtk::Button::new();
                    let uri = item.uri_display().unwrap().to_string().to_variant();
                    //row.set_activatable_widget(Some(&button));
                    row.set_title(&item.display_name());
                    row.add_suffix(&button);
                    button.add_css_class("flat");
                    button.set_vexpand(false);
                    button.set_valign(gtk::Align::Center);
                    button.set_icon_name("go-next-symbolic");
                    if let Some(uri) = item.uri_display() {
                        row.set_subtitle(&uri)
                    };
                    if item.mime_type().starts_with("text") {
                        files.append(&row);
                        row.set_icon_name(Some(&sidebar::icons::from_mime_type(
                            item.mime_type().into(),
                        )));
                        // FIXME gtk bug(?): connect_activate doesn't work anymore???
                        button.connect_clicked(move |b| {
                            b.activate_action("win.tab.new", Some(&uri)).unwrap();
                        });
                    } else if item.mime_type() == "inode/directory" {
                        folders.append(&row);
                        row.set_icon_name(Some("folder-symbolic"));
                        button.connect_clicked(clone!(
                        #[strong]
                        this,move |b| {
                            b.activate_action("win.set_folder", Some(&uri))
                                .unwrap();
                                adw::prelude::AdwDialogExt::close(&this);
                        }));
                    }
                }
            });
        }
    }
    impl WidgetImpl for RecentsDialog {}
    impl WindowImpl for RecentsDialog {}
    impl DialogImpl for RecentsDialog {}
    impl AdwDialogImpl for RecentsDialog {}
}

glib::wrapper! {
pub struct RecentsDialog(ObjectSubclass<imp::RecentsDialog>)
    @extends adw::Dialog, gtk::Dialog, gtk::Window, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl RecentsDialog {
    pub fn new() -> Self {
        glib::Object::builder::<Self>().build()
    }

    pub fn present(&self, window: &gtk::Window) {
        AdwDialogExt::present(self, Some(window));
    }
}
