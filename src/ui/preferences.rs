use crate::config::Config;
use crate::utils;
use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;
use gtk::glib::clone;

mod imp {

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/preferences.blp")]
    #[properties(wrapper_type = super::PreferencesDialog)]
    pub struct PreferencesDialog {
        #[template_child]
        pub colorschemes: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub theme_variant: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub line_hl: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub minimap: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub grid_bg: TemplateChild<adw::SwitchRow>,
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

macro_rules! connect_switch {
    ($switch:expr, $cfg:ident.$setting:ident.$key:ident) => {
        $switch.set_active($cfg.$setting.$key);
        $switch.connect_active_notify(|s| {
            let mut config = Config::get_or_default();
            config.$setting.$key = s.is_active();
            config.write();
            s.activate_action("win.reload_config", None).unwrap();
        });
    };
}

impl PreferencesDialog {
    pub fn new() -> Self {
        glib::Object::builder::<Self>().build()
    }

    pub fn present(&self, window: &gtk::Window) {
        let config = Config::get_or_default();
        self.update_colorschemes(&config);
        // Connect thingies
        connect_switch!(self.imp().line_hl, config.ui.hl_line);
        connect_switch!(self.imp().minimap, config.ui.show_minimap);
        connect_switch!(self.imp().grid_bg, config.ui.grid_bg);
        // NOTE position of the strings in the blueprint file matters for this one lmao
        self.imp().theme_variant.set_selected(match config.ui.theme_variant {
            crate::config::ThemeVariant::System => 0,
            crate::config::ThemeVariant::Light => 1,
            crate::config::ThemeVariant::Dark => 2,
        });
        self.imp().theme_variant.connect_selected_notify(|theme_variant| {
            let mut config = Config::get_or_default();
            config.ui.theme_variant = match theme_variant.selected() {
                1 => crate::config::ThemeVariant::Light,
                2 => crate::config::ThemeVariant::Dark,
                _ => crate::config::ThemeVariant::System,
            };
            config.write();
            theme_variant.activate_action("win.reload_config", None).unwrap();
        });
        // Present
        AdwDialogExt::present(self, Some(window));
    }

    fn update_colorschemes(&self, config: &Config) {
        let scheme_manager = sourceview5::StyleSchemeManager::new();
        let flow_box = self.imp().colorschemes.clone();
        flow_box.remove_all();
        for scheme in scheme_manager.scheme_ids() {
            if let Some(ref scheme) = scheme_manager.scheme(&scheme)
                && (
                    utils::scheme::is_dark(scheme).is_none() // Theme doesn't have light/dark variants
                || (utils::scheme::is_dark(scheme).unwrap() && adw::StyleManager::default().is_dark()) // OR it is and it matches the dark theme
                || (!utils::scheme::is_dark(scheme).unwrap() && !adw::StyleManager::default().is_dark())
                    // OR it isn't and the system has a light theme
                )
            {
                let flow_child = gtk::FlowBoxChild::new();
                let widget = sourceview5::StyleSchemePreview::new(scheme);
                if utils::scheme::light(scheme).id() == config.ui.theme {
                    widget.set_selected(true);
                }
                widget.add_css_class("card");
                widget.connect_activate(clone!(
                    #[strong]
                    config,
                    #[strong]
                    scheme,
                    #[strong]
                    flow_box,
                    move |widget| {
                        let mut config = config.clone();
                        let mut child = flow_box.first_child();
                        while let Some(curr) = child {
                            curr.downcast_ref::<gtk::FlowBoxChild>()
                                .unwrap()
                                .child()
                                .unwrap()
                                .downcast_ref::<sourceview5::StyleSchemePreview>()
                                .unwrap()
                                .set_selected(false);
                            child = curr.next_sibling();
                        }
                        config.ui.theme = utils::scheme::light(&scheme).id().to_string();
                        config.write();
                        widget.set_selected(true);
                        widget.activate_action("win.reload_config", None).unwrap();
                    }
                ));
                flow_child.set_child(Some(&widget));
                flow_box.append(&flow_child);
            }
        }
    }
}
