use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::gdk::RGBA;
use gtk::glib;
use vte4::Terminal;
use vte4::prelude::*;

mod imp {

    use gtk::{gio::Cancellable, glib::SpawnFlags};
    use vte4::PtyFlags;

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/terminal.blp")]
    #[properties(wrapper_type = super::TerminalView)]
    pub struct TerminalView {
        #[template_child]
        pub terminal: TemplateChild<Terminal>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TerminalView {
        const NAME: &'static str = "TerminalView";
        type Type = super::TerminalView;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TerminalView {
        fn constructed(&self) {
            self.parent_constructed();
            self.terminal.spawn_async(
                PtyFlags::DEFAULT,
                None,
                &[&std::env::var("SHELL").unwrap_or("/bin/sh".to_string())],
                &[],
                SpawnFlags::empty(),
                || {},
                10,
                Some(&Cancellable::new()),
                |_| {},
            );
            if let Some(ref scheme) = sourceview5::StyleSchemeManager::new().scheme("Adwaita") {
                self.obj().set_palette(scheme);
            }
        }
    }
    impl WidgetImpl for TerminalView {}
    impl BoxImpl for TerminalView {}
}

glib::wrapper! {
pub struct TerminalView(ObjectSubclass<imp::TerminalView>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

macro_rules! get_color {
    ($scheme:ident, $style:expr, $color:ident) => {
        if let Some(scheme) = $scheme.style($style)
            && let Some(color) = scheme.$color()
            && let Ok(rgba) = RGBA::parse(color)
        {
            Some(rgba)
        } else {
            None
        }
    };
}

impl TerminalView {
    fn set_palette(&self, scheme: &sourceview5::StyleScheme) {
        // Custom color definitions with a hard-coded fallback (adwaita light theme)
        let foreground =
            get_color!(scheme, "term:fg", foreground).unwrap_or(RGBA::parse("#c0bfbc").unwrap());
        let background =
            get_color!(scheme, "term:bg", foreground).unwrap_or(RGBA::parse("#1e1e1e").unwrap());
        let black =
            get_color!(scheme, "term:black", foreground).unwrap_or(RGBA::parse("#5d5d5d").unwrap());
        let red =
            get_color!(scheme, "term:red", foreground).unwrap_or(RGBA::parse("#f66151").unwrap());
        let green =
            get_color!(scheme, "term:green", foreground).unwrap_or(RGBA::parse("#33d17a").unwrap());
        let yellow = get_color!(scheme, "term:yellow", foreground)
            .unwrap_or(RGBA::parse("#e9ad0c").unwrap());
        let blue =
            get_color!(scheme, "term:blue", foreground).unwrap_or(RGBA::parse("#2a7bde").unwrap());
        let magenta = get_color!(scheme, "term:magenta", foreground)
            .unwrap_or(RGBA::parse("#c061cb").unwrap());
        let teal =
            get_color!(scheme, "term:teal", foreground).unwrap_or(RGBA::parse("#33c7de").unwrap());
        let white =
            get_color!(scheme, "term:white", foreground).unwrap_or(RGBA::parse("#ffffff").unwrap());

        // Set colors
        self.imp().terminal.set_colors(
            Some(&foreground),
            Some(&background),
            &[
                &black, &red, &green, &yellow, &blue, &magenta, &teal, &white,
            ],
        );
    }
}
