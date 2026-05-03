use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib::{self, clone};
use std::cell::RefCell;
use adw::prelude::*;

mod imp {

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/file_name_dialog.blp")]
    #[properties(wrapper_type = super::FileNameDialog)]
    pub struct FileNameDialog {
        #[template_child]
        pub entry: TemplateChild<gtk::Entry>,
        pub kind: RefCell<super::Kind>,
        pub path: RefCell<Option<String>>
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
            self.entry.connect_activate(clone!(
                #[weak(rename_to=this)]
                self,
                move |entry| {
                    let binding = entry.text();
                    let filename = binding.trim();
                    if !filename.is_empty() {
                        match *this.kind.borrow() {
                            super::Kind::Rename => println!("rename"),
                            super::Kind::Create => {std::fs::File::create(format!("{}/{}", this.path.borrow().clone().unwrap(), filename)).unwrap();}
                        }
                        adw::prelude::AdwDialogExt::close(&this.obj().clone());
                    }
            }));
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
    Create
}

impl FileNameDialog {
    pub fn new() -> Self {
            glib::Object::builder::<Self>().build()
    }
    
    pub fn set_kind(&self, kind: Kind) {
        self.imp().kind.replace(kind);
    }
    
    pub fn set_path(&self, path: &str) {
        self.imp().path.replace(Some(path.to_string()));
    }
}
