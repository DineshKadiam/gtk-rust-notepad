use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib::{self, clone};
use std::cell::RefCell;
use std::path::Path;

mod imp {

    use std::path::Path;

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/file_name_dialog.blp")]
    #[properties(wrapper_type = super::FileNameDialog)]
    pub struct FileNameDialog {
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub apply_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub entry: TemplateChild<gtk::Entry>,

        pub kind: RefCell<super::Kind>,
        pub path: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileNameDialog {
        const NAME: &'static str = "FileNameDialog";
        type Type = super::FileNameDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FileNameDialog {
        fn constructed(&self) {
            self.parent_constructed();
            // AAAAAAA (this just sets the button to be disabled by default. nothing else works)
            self.apply_button.connect_sensitive_notify(clone!(
                #[weak(rename_to=entry)]
                self.entry,
                move |btn| {
                    btn.set_sensitive(btn.get_sensitive() && !entry.text().is_empty());
                }
            ));
            // Hitting enter on the entry activates the "Apply" button
            self.entry.connect_activate(clone!(
                #[weak(rename_to=btn)]
                self.apply_button,
                move |_| {
                    btn.activate();
                }
            ));
            // Typing a valid string makes the apply button sensitive
            self.entry.connect_changed(clone!(
                #[weak(rename_to=btn)]
                self.apply_button,
                move |entry| {
                    btn.set_sensitive(
                        !entry.text().trim().is_empty() && !entry.text().contains("/"),
                    );
                }
            ));
            // Connect the apply button
            self.apply_button.connect_activate(clone!(
                #[weak(rename_to=this)]
                self,
                move |_| {
                    let binding = this.entry.text();
                    let filename = binding.trim();
                    let mut path = Path::new(&this.path.borrow().clone().unwrap()).to_path_buf();
                    if !filename.is_empty() {
                        match *this.kind.borrow() {
                            super::Kind::Rename => {
                                std::fs::rename(
                                    this.path.borrow().clone().unwrap(),
                                    path.parent().unwrap().join(filename).to_str().unwrap(),
                                )
                                .unwrap();
                            }
                            super::Kind::Create => {
                                if path.is_file() {
                                    path = path.parent().unwrap().to_path_buf();
                                }
                                std::fs::File::create(path.join(filename).to_str().unwrap())
                                    .unwrap();
                            }
                        }
                        adw::prelude::AdwDialogExt::close(&this.obj().clone());
                    }
                }
            ));
        }
    }
    impl WidgetImpl for FileNameDialog {}
    impl WindowImpl for FileNameDialog {}
    impl DialogImpl for FileNameDialog {}
    impl AdwDialogImpl for FileNameDialog {}
}

glib::wrapper! {
pub struct FileNameDialog(ObjectSubclass<imp::FileNameDialog>)
    @extends adw::Dialog, gtk::Dialog, gtk::Window, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[derive(Default, Debug)]
pub enum Kind {
    #[default]
    Rename,
    Create,
}

impl FileNameDialog {
    pub fn new(kind: Kind, path: &str) -> Self {
        let dialog = glib::Object::builder::<Self>().build();
        dialog.imp().title.set_label(match kind {
            Kind::Rename => "Rename file…",
            Kind::Create => "Create file…",
        });
        match kind {
            Kind::Rename => {
                let path = Path::new(path).to_path_buf();
                dialog
                    .imp()
                    .entry
                    .set_text(path.file_name().unwrap().to_str().unwrap());
            }
            _ => (),
        }
        dialog.imp().kind.replace(kind);
        dialog.imp().path.replace(Some(path.to_string()));
        dialog
    }
}
