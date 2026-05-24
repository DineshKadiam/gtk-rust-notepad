use adw::subclass::prelude::*;
use glib::Properties;
use gtk::gdk::RGBA;
use gtk::gio::Cancellable;
use gtk::glib;
use gtk::glib::SpawnFlags;
use gtk::CompositeTemplate;
use vte4::prelude::*;
use vte4::PtyFlags;
use vte4::Terminal;

use crate::utils;

const DEFAULT_SCHEME_DARK: &str = include_str!("../../resources/styles/TERM_DEFAULT_DARK");
const DEFAULT_SCHEME_LIGHT: &str = include_str!("../../resources/styles/TERM_DEFAULT_LIGHT");

mod imp {

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
        let fallback = if utils::scheme::is_dark(scheme).is_some_and(|d| d) {
            DEFAULT_SCHEME_DARK
        } else {
            DEFAULT_SCHEME_LIGHT
        }
        .split("\n")
        .map(|color| {
            if color.len() >= 7 {
                RGBA::parse(&color[..7]).unwrap()
            } else {
                RGBA::TRANSPARENT
            }
        })
        .collect::<Vec<RGBA>>();
        // Custom color definitions with a hard-coded fallback (adwaita light theme)
        let foreground = get_color!(scheme, "term:fg").unwrap_or((fallback[0], RGBA::TRANSPARENT));
        let background =
            get_color!(scheme, "term:bg").unwrap_or((RGBA::TRANSPARENT, RGBA::TRANSPARENT));
        let black = get_color!(scheme, "term:black").unwrap_or((fallback[2], fallback[3]));
        let red = get_color!(scheme, "term:red").unwrap_or((fallback[4], fallback[5]));
        let green = get_color!(scheme, "term:green").unwrap_or((fallback[6], fallback[7]));
        let yellow = get_color!(scheme, "term:yellow").unwrap_or((fallback[8], fallback[9]));
        let blue = get_color!(scheme, "term:blue").unwrap_or((fallback[10], fallback[11]));
        let magenta = get_color!(scheme, "term:magenta").unwrap_or((fallback[12], fallback[13]));
        let teal = get_color!(scheme, "term:teal").unwrap_or((fallback[14], fallback[15]));
        let white = get_color!(scheme, "term:white").unwrap_or((fallback[16], fallback[17]));

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

impl TerminalView {
    pub fn new() -> Self {
        glib::Object::builder::<Self>().build()
    }
}
