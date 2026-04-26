use adw::subclass::prelude::*;
use async_lsp::lsp_types::DiagnosticSeverity;
use async_lsp::lsp_types::Range;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::TextSearchFlags;
use gtk::gio::Icon;
use gtk::glib;
use gtk::glib::Variant;
use gtk::pango::Underline;
use sourceview5::Annotation;
use sourceview5::AnnotationProvider;
use sourceview5::BackgroundPatternType;
use sourceview5::StyleSchemeManager;
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::io::Error;

use crate::config::Config;
use crate::utils;

mod imp {

    use gtk::{EventControllerMotion, Revealer, glib::clone};

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[template(file = "resources/ui/tab.blp")]
    #[properties(wrapper_type = super::Buffer)]
    pub struct Buffer {
        #[template_child]
        pub source_view: TemplateChild<sourceview5::View>,
        #[template_child]
        pub source_map: TemplateChild<sourceview5::Map>,
        #[template_child]
        pub map_revealer: TemplateChild<Revealer>,
        #[property(get, set)]
        pub file_path: RefCell<String>,
        #[property(get, set)]
        pub diagnostic_provider: RefCell<AnnotationProvider>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Buffer {
        const NAME: &'static str = "Buffer";
        type Type = super::Buffer;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Buffer {
        fn constructed(&self) {
            self.parent_constructed();
            // Show map on mouse motion only
            let motion_controller = EventControllerMotion::new();
            motion_controller.connect_enter(clone!(
                #[strong(rename_to=revealer)]
                self.map_revealer,
                move |_controller, _, _| {
                    revealer.set_reveal_child(true);
                }
            ));
            motion_controller.connect_leave(clone!(
                #[strong(rename_to=revealer)]
                self.map_revealer,
                move |_controller| {
                    revealer.set_reveal_child(false);
                }
            ));
            self.obj().add_controller(motion_controller);
        }
    }
    impl WidgetImpl for Buffer {}
    impl BoxImpl for Buffer {}
}

glib::wrapper! {
pub struct Buffer(ObjectSubclass<imp::Buffer>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

pub enum SearchDirection {
    Forward,
    Back,
    Cursor,
}

impl Buffer {
    pub fn new() -> Self {
        let widget = glib::Object::builder::<Self>().build();
        widget
            .imp()
            .source_view
            .annotations()
            .add_provider(&widget.imp().diagnostic_provider.borrow().clone());
        widget
    }

    /// Creates a buffer ui/tab thingy with a SourceView Buffer object
    pub fn with_source(buf: sourceview5::Buffer) -> Self {
        let widget = glib::Object::builder::<Self>().build();
        widget.imp().source_view.set_buffer(Some(&buf));
        widget
            .imp()
            .source_view
            .annotations()
            .add_provider(&widget.imp().diagnostic_provider.borrow().clone());
        widget
    }

    /// Clear diagnostics
    pub fn clear_diagnostics(&self) {
        self.imp().diagnostic_provider.borrow().remove_all();
        // Clear tags
        let buffer = self
            .imp()
            .source_view
            .buffer()
            .downcast_ref::<sourceview5::Buffer>()
            .unwrap()
            .clone();
        let bounds = buffer.bounds();
        {
            let mut exists = false;
            buffer.tag_table().foreach(|t| {
                if t.name().is_some_and(|n| n == "diag_error") {
                    exists = true;
                }
            });
            if exists {
                buffer.remove_tag_by_name("diag_error", &bounds.0, &bounds.1);
            }
        }
    }

    /// Highlight diaganostic
    pub fn highlight_diagnostic(
        &self,
        range: &Range,
        message: String,
        severity: DiagnosticSeverity,
        create_annotation: bool,
    ) {
        // Add annotation
        if create_annotation {
            self.imp()
                .diagnostic_provider
                .borrow()
                .add_annotation(&Annotation::new(
                    Some(&message),
                    Some(
                        Icon::for_string(match severity {
                            DiagnosticSeverity::ERROR => "dialog-error-symbolic",
                            DiagnosticSeverity::WARNING => "dialog-warning-symbolic",
                            _ => "dialog-information-symbolic",
                        })
                        .unwrap(),
                    ),
                    range.start.line as i32,
                    match severity {
                        DiagnosticSeverity::ERROR => sourceview5::AnnotationStyle::Error,
                        DiagnosticSeverity::WARNING => sourceview5::AnnotationStyle::Warning,
                        _ => sourceview5::AnnotationStyle::Accent,
                    },
                ));
        }
        // Add tag
        let buffer = self
            .imp()
            .source_view
            .buffer()
            .downcast_ref::<sourceview5::Buffer>()
            .unwrap()
            .clone();
        if let Some(start) =
            buffer.iter_at_line_offset(range.start.line as i32, range.start.character as i32)
            && let Some(end) =
                buffer.iter_at_line_offset(range.end.line as i32, range.end.character as i32)
        {
            let mut exists = false;
            buffer.tag_table().foreach(|t| {
                if t.name().is_some_and(|n| n == "diag_error") {
                    exists = true;
                }
            });
            if !exists {
                buffer.create_tag(Some("diag_error"), &[("underline", &Underline::Error)]);
            }
            buffer.apply_tag_by_name("diag_error", &start, &end);
        }
    }

    /// Sets information in the statusbar
    pub fn set_status(&self, workspace_dir: &str) {
        let buffer = self
            .imp()
            .source_view
            .buffer()
            .downcast_ref::<sourceview5::Buffer>()
            .unwrap()
            .clone();
        let path = if self.file_path().is_empty() {
            "[scratch]".to_string()
        } else {
            self.file_path()
                .replace(workspace_dir, "")
                .trim_start_matches("/")
                .to_string()
        };
        let lang = if let Some(lang) = buffer.language() {
            lang.name().to_string()
        } else {
            "text".to_string()
        };
        let variant = Variant::tuple_from_iter(vec![
            path.to_variant(),
            lang.to_variant(),
            true.to_variant(),
        ]);
        self.activate_action("win.status.set_file", Some(&variant))
            .unwrap();
    }

    /// Update self based on config file
    pub fn configure(&self) {
        let config = Config::get_or_default();
        let buffer = self
            .imp()
            .source_view
            .buffer()
            .downcast_ref::<sourceview5::Buffer>()
            .unwrap()
            .clone();
        if let Some(ref scheme) = StyleSchemeManager::new().scheme(&config.ui.theme) {
            if adw::StyleManager::default().is_dark() {
                buffer.set_style_scheme(Some(&utils::scheme::dark(scheme)));
            } else {
                buffer.set_style_scheme(Some(scheme));
            }
        }
        self.imp().map_revealer.set_visible(config.ui.show_minimap);
        self.imp()
            .source_view
            .set_highlight_current_line(config.ui.hl_line);
        self.imp()
            .source_view
            .set_background_pattern(if config.ui.grid_bg {
                BackgroundPatternType::Grid
            } else {
                BackgroundPatternType::None
            });
    }

    pub fn search(&self, query: &str, direction: SearchDirection) -> Result<(), Error> {
        let buffer = self
            .imp()
            .source_view
            .buffer()
            .downcast_ref::<sourceview5::Buffer>()
            .unwrap()
            .clone();
        let bounds = buffer.bounds();
        let mut cursor_pos = bounds.0;
        cursor_pos.forward_chars(match direction {
            SearchDirection::Forward => buffer.cursor_position() + 1,
            SearchDirection::Back => buffer.cursor_position(),
            SearchDirection::Cursor => buffer.cursor_position(),
        });

        if let Some(bounds) = buffer.selection_bounds() {
            cursor_pos = match direction {
                SearchDirection::Forward => bounds.1,
                SearchDirection::Back => bounds.0,
                SearchDirection::Cursor => bounds.0,
            }
        }

        // Search text
        let text_iter = match direction {
            SearchDirection::Forward => {
                if let Some(text_iter) =
                    cursor_pos.forward_search(query, TextSearchFlags::empty(), Some(&bounds.1))
                {
                    Some(text_iter)
                } else {
                    bounds
                        .0
                        .forward_search(query, TextSearchFlags::empty(), Some(&bounds.1))
                }
            }
            SearchDirection::Cursor => {
                if let Some(text_iter) =
                    cursor_pos.forward_search(query, TextSearchFlags::empty(), Some(&bounds.1))
                {
                    Some(text_iter)
                } else {
                    bounds
                        .0
                        .forward_search(query, TextSearchFlags::empty(), Some(&bounds.1))
                }
            }
            SearchDirection::Back => {
                if let Some(text_iter) =
                    cursor_pos.backward_search(query, TextSearchFlags::empty(), Some(&bounds.0))
                {
                    Some(text_iter)
                } else {
                    bounds
                        .1
                        .backward_search(query, TextSearchFlags::empty(), Some(&bounds.0))
                }
            }
        };
        if let Some(text_iter) = text_iter {
            let mut start = text_iter.0;
            buffer.place_cursor(&text_iter.0);
            buffer.select_range(&text_iter.0, &text_iter.1);
            self.imp()
                .source_view
                .scroll_to_iter(&mut start, 0.0, true, 0.5, 0.0);
            Ok(())
        } else {
            Err(std::io::Error::other("String not found"))
        }
    }
}
