use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;
use sourceview5::StyleScheme;

mod imp {

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/preferences.blp")]
    #[properties(wrapper_type = super::PreferencesDialog)]
    pub struct PreferencesDialog {
        #[template_child]
        pub colorschemes: TemplateChild<gtk::FlowBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesDialog {
        const NAME: &'static str = "PreferencesDialog";
        type Type = super::PreferencesDialog;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PreferencesDialog {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for PreferencesDialog {}
    impl WindowImpl for PreferencesDialog {}
    impl DialogImpl for PreferencesDialog {}
    impl AdwDialogImpl for PreferencesDialog {}
    impl PreferencesDialogImpl for PreferencesDialog {}
}

glib::wrapper! {
pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
    @extends adw::PreferencesDialog, adw::Dialog, gtk::Dialog, gtk::Window, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl PreferencesDialog {
    pub fn new() -> Self {
        glib::Object::builder::<Self>().build()
    }

    pub fn present(&self, window: &gtk::Window) {
        // Fill colorschemes
        let scheme_manager = sourceview5::StyleSchemeManager::new();
        let flow_box = self.imp().colorschemes.clone();
        for scheme in scheme_manager.scheme_ids() {
            if let Some(ref scheme) = scheme_manager.scheme(&scheme)
                && (
                    style_scheme_is_dark(scheme).is_none() // Theme doesn't have light/dark variants
                || (style_scheme_is_dark(scheme).unwrap() && adw::StyleManager::default().is_dark()) // OR it is and it matches the dark theme
                || (!style_scheme_is_dark(scheme).unwrap() && !adw::StyleManager::default().is_dark())
                    // OR it isn't and the system has a light theme
                )
            {
                let flow_child = gtk::FlowBoxChild::new();
                let widget = sourceview5::StyleSchemePreview::new(scheme);
                if &scheme.id() == "Adwaita" {
                    widget.set_selected(true);
                }
                widget.add_css_class("card");
                flow_child.set_child(Some(&widget));
                flow_box.append(&flow_child);
            }
        }
        // Present
        AdwDialogExt::present(self, Some(window));
    }
}

// TODO put in global utils file??
fn style_scheme_is_dark(scheme: &StyleScheme) -> Option<bool> {
    if let Some(variant) = scheme.metadata("variant") {
        Some(variant.contains("dark"))
    } else if scheme.name().contains("dark") {
        Some(true)
    } else {
        None
    }
}
