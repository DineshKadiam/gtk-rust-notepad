use adw::{ActionRow, prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::{CompositeTemplate, glib, glib::clone};

use crate::{config::Config, ui::sidebar};

mod imp {

    use gtk::ListBox;
    use itertools::Itertools;

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/recents.blp")]
    #[properties(wrapper_type = super::RecentsDialog)]
    pub struct RecentsDialog {
        #[template_child]
        pub starred_status: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub starred_files: TemplateChild<ListBox>,
        #[template_child]
        pub starred_folders: TemplateChild<ListBox>,
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

            let config = Config::get_or_default();
            let files = self.files.clone();
            let folders = self.folders.clone();
            let starred_files = self.starred_files.clone();
            let starred_folders = self.starred_folders.clone();
            let starred_status = self.starred_status.clone();
            let this = self.obj().clone();

            // Not a ListModel so I can't use ListView... damn...
            gtk::RecentManager::default()
                .items()
                .iter()
                .sorted_by(|a, b| b.modified().cmp(&a.modified()))
                .filter(|item| {
                    item.exists()
                        && item.is_local()
                        && item.applications().contains(&"ky".to_string().into())
                })
                .for_each(|item| {
                    let uri = item.uri_display().unwrap().to_string();
                    let row = this.create_row(
                        uri.clone(),
                        item.mime_type().to_string(),
                        item.display_name().to_string(),
                        &config,
                    );
                    if item.mime_type().starts_with("text") {
                        files.append(&row);
                    } else if item.mime_type() == "inode/directory" {
                        folders.append(&row);
                    }
                    // Starred
                    if config.pinned_files.contains(&uri) {
                        let row = this.create_row(
                            uri.clone(),
                            item.mime_type().to_string(),
                            item.display_name().to_string(),
                            &config,
                        );
                        starred_status.set_visible(false);
                        if item.mime_type().starts_with("text") {
                            starred_files.append(&row);
                        } else if item.mime_type() == "inode/directory" {
                            starred_folders.append(&row);
                        }
                    }
                });

            // Make the UI look pretty if you haven't starred anything (yet!)
            self.starred_files
                .parent()
                .unwrap()
                .set_visible(self.starred_files.observe_children().n_items() > 0);
            self.starred_folders
                .parent()
                .unwrap()
                .set_visible(self.starred_folders.observe_children().n_items() > 0);
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

    /// Lotta boilerplate which creates a row for a file/folder; contains the display name, file path, a button to star/pin/favourite
    /// and a button to open it.
    fn create_row(
        &self,
        uri: String,
        mime_type: String,
        name: String,
        config: &Config,
    ) -> ActionRow {
        let uri_v = uri.clone().to_variant();
        let row = adw::ActionRow::builder().title(name).subtitle(&uri).build();
        let button = gtk::Button::builder()
            .css_classes(["flat"])
            .vexpand(false)
            .valign(gtk::Align::Center)
            .icon_name("external-link-symbolic")
            .build();
        let star = gtk::ToggleButton::builder()
            .css_classes(["flat", "transparent"])
            .vexpand(false)
            .valign(gtk::Align::Center)
            .icon_name(if config.pinned_files.contains(&uri) {
                "starred"
            } else {
                "non-starred"
            })
            .active(config.pinned_files.contains(&uri))
            .build();
        row.add_suffix(&star);
        row.add_suffix(&button);
        row.add_prefix(&gtk::Image::from_gicon(&sidebar::icons::from_mime_type(
            mime_type.clone(),
        )));
        // Onclick events
        // FIXME gtk bug(?): connect_activate doesn't work anymore???
        star.connect_clicked(move |b| {
            let mut config = Config::get_or_default();
            b.set_icon_name(if b.is_active() {
                "starred"
            } else {
                "non-starred"
            });
            if b.is_active() {
                config.pinned_files.push(uri.clone());
            } else if let Some(index) = config.pinned_files.iter().position(|f| f == &uri) {
                config.pinned_files.remove(index);
            }
            config.write();
        });
        if mime_type.starts_with("text") {
            button.connect_clicked(move |b| {
                b.activate_action("win.tab.new", Some(&uri_v)).unwrap();
            });
        } else if mime_type == "inode/directory" {
            button.connect_clicked(clone!(
                #[strong(rename_to=this)]
                self,
                move |b| {
                    b.activate_action("win.set_folder", Some(&uri_v)).unwrap();
                    adw::prelude::AdwDialogExt::close(&this);
                }
            ));
        }
        row
    }
}
