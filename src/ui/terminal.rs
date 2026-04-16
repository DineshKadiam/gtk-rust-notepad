use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::gdk::RGBA;
use gtk::gio::Cancellable;
use gtk::glib;
use gtk::glib::SpawnFlags;
use vte4::PtyFlags;
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
            // TODO fixme
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
    ($scheme:ident, $style:expr) => {
        if let Some(scheme) = $scheme.style($style)
            && let Some(foreground) = scheme.foreground()
            && let Some(background) = scheme.background()
        {
            Some((
                RGBA::parse(foreground).unwrap(),
                RGBA::parse(background).unwrap(),
            ))
        } else {
            None
        }
    };
}

impl TerminalView {
    pub fn setup(&self, folder: &str) {
        self.imp().terminal.reset(true, true);
        self.imp().terminal.spawn_async(
            PtyFlags::DEFAULT,
            Some(folder),
            &[&std::env::var("SHELL").unwrap_or("/bin/sh".to_string())],
            &[],
            SpawnFlags::empty(),
            || {},
            10,
            Some(&Cancellable::new()),
            |_| {},
        );
    }

    pub fn set_palette(&self, scheme: &sourceview5::StyleScheme) {
        // Custom color definitions with a hard-coded fallback (adwaita light theme)
        let foreground = get_color!(scheme, "term:fg")
            .unwrap_or((RGBA::parse("#c0bfbc").unwrap(), RGBA::TRANSPARENT));
        let background = get_color!(scheme, "term:bg")
            .unwrap_or((RGBA::parse("#1d1d20").unwrap(), RGBA::TRANSPARENT));
        let black = get_color!(scheme, "term:black").unwrap_or((
            RGBA::parse("#5d5d5d").unwrap(),
            RGBA::parse("#1e1e1e").unwrap(),
        ));
        let red = get_color!(scheme, "term:red").unwrap_or((
            RGBA::parse("#f66151").unwrap(),
            RGBA::parse("#c01c28").unwrap(),
        ));
        let green = get_color!(scheme, "term:green").unwrap_or((
            RGBA::parse("#33d17a").unwrap(),
            RGBA::parse("#26a269").unwrap(),
        ));
        let yellow = get_color!(scheme, "term:yellow").unwrap_or((
            RGBA::parse("#e9ad0c").unwrap(),
            RGBA::parse("#a2734c").unwrap(),
        ));
        let blue = get_color!(scheme, "term:blue").unwrap_or((
            RGBA::parse("#2a7bde").unwrap(),
            RGBA::parse("#12488b").unwrap(),
        ));
        let magenta = get_color!(scheme, "term:magenta").unwrap_or((
            RGBA::parse("#c061cb").unwrap(),
            RGBA::parse("#a347ba").unwrap(),
        ));
        let teal = get_color!(scheme, "term:teal").unwrap_or((
            RGBA::parse("#33c7de").unwrap(),
            RGBA::parse("#2aa1b3").unwrap(),
        ));
        let white = get_color!(scheme, "term:white").unwrap_or((
            RGBA::parse("#ffffff").unwrap(),
            RGBA::parse("#cfcfcf").unwrap(),
        ));

        // Set colors
        self.imp().terminal.set_colors(
            Some(&foreground.0),
            Some(&background.0),
            &[
                &black.1, &red.1, &green.1, &yellow.1, &blue.1, &magenta.1, &teal.1, &white.1,
                &black.0, &red.0, &green.0, &yellow.0, &blue.0, &magenta.0, &teal.0, &white.0,
            ],
        );
    }
}
