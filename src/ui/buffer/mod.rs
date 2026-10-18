use std::{
    cell::RefCell,
    fmt::Debug,
    fs,
    io::Error,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use adw::{AlertDialog, ResponseAppearance, prelude::*, subclass::prelude::*};
use async_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};
use gtk::{
    CompositeTemplate, CssProvider, EventControllerMotion, Revealer, TextSearchFlags,
    gio::{self, Cancellable, FileQueryInfoFlags},
    glib::{self, ControlFlow, Properties, SourceId, clone},
    pango::{FontDescription, FontMap, Underline, ffi::PANGO_SCALE},
};
use sourceview5::{BackgroundPatternType, FileSaverFlags, StyleSchemeManager, prelude::*};

use crate::{
    config::Config,
    lsp,
    ui::{completion::provider::LspCompletionProvider, sidebar, window::EditorWindow},
    utils,
};

mod imp;

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
        glib::Object::builder::<Self>().build()
    }

    /// Creates a buffer ui/tab thingy with a SourceView Buffer object
    pub fn with_source(buf: sourceview5::Buffer) -> Self {
        let widget = glib::Object::builder::<Self>().build();
        widget.imp().source_view.set_buffer(Some(&buf));
        widget.setup_annotations();
        // Scroll to the bottom when new lines are added; with a margin (unlike by default)
        Buffer::override_scrolling(
            widget.get_sourceview_buffer(),
            widget.imp().source_view.clone(),
        );
        widget
    }

    /// Gets a sanitized, readable name for the buffer -- its path relative to the workspace directory.
    pub fn clean_name(&self, workspace_dir: &str) -> String {
        // TODO we can actually store this, and eliminate the need to borrow a workspace dir EVERY time
        // store in Buffer::name()
        if let Some(path) = self.file_path() {
            path.replace(workspace_dir, "")
                .trim_start_matches("/")
                .to_string()
        } else {
            "[scratch]".to_string()
        }
    }

    /// Gets the short name of the buffer, plus a string if it was modified.
    pub fn tab_name(&self) -> String {
        (if let Some(path) = self.file_path() {
            utils::basename(&path).to_string()
        } else {
            "scratch".to_string()
        }) + if self.get_sourceview_buffer().is_modified() {
            " [+]"
        } else {
            ""
        }
    }

    /// Returns the sourceview5 buffer from self.source_view
    pub fn get_sourceview_buffer(&self) -> sourceview5::Buffer {
        self.imp()
            .source_view
            .buffer()
            .downcast_ref::<sourceview5::Buffer>()
            .unwrap()
            .clone()
    }

    /// Sets information in the statusbar
    pub fn set_status(&self, workspace_dir: &str) {
        let buffer = self.get_sourceview_buffer();
        let path = self.clean_name(workspace_dir);
        let lang = if let Some(lang) = buffer.language() {
            lang.name().to_string()
        } else {
            "text".to_string()
        };
        self.activate_action(
            "win.status.set_file",
            Some(&vec![path.to_variant(), lang.to_variant(), "yes".to_variant()].into()),
        )
        .unwrap();
    }

    /// Update self based on config file
    pub fn configure(&self) {
        let config = Config::get_or_default();
        let buffer = self.get_sourceview_buffer();
        let font = FontDescription::from_string(&config.ui.font);
        if let Some(ref scheme) = StyleSchemeManager::new().scheme(&config.ui.theme) {
            if adw::StyleManager::default().is_dark() {
                buffer.set_style_scheme(Some(&utils::scheme::dark(scheme)));
            } else {
                buffer.set_style_scheme(Some(scheme));
            }
        }
        self.css_provider().load_from_string(&format!(
            "textview {{ font: {} {}; {} }}",
            if font.size() > 0 {
                format!("{}pt", font.size() / PANGO_SCALE)
            } else {
                String::new()
            },
            font.family().unwrap(),
            match font.weight() {
                gtk::pango::Weight::Thin => "font-weight: thin",
                gtk::pango::Weight::Light => "font-weight: light",
                gtk::pango::Weight::Bold => "font-weight: bold",
                _ => "",
            }
        ));
        self.imp()
            .source_view
            .set_highlight_current_line(config.ui.hl_line);
        self.imp()
            .source_view
            .set_wrap_mode(if config.ui.word_wrap {
                gtk::WrapMode::Word
            } else {
                gtk::WrapMode::None
            });
        self.imp()
            .source_view
            .set_background_pattern(if config.ui.grid_bg {
                BackgroundPatternType::Grid
            } else {
                BackgroundPatternType::None
            });
    }

    pub fn search(&self, query: &str, direction: SearchDirection) -> Result<(), Error> {
        let buffer = self.get_sourceview_buffer();
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

    pub fn update_uri(&self, uri: &str, workspace_uri: &str) {
        // Replace inner workspace URI
        self.imp().workspace_uri.replace(workspace_uri.to_string());
        // Replace completion provider workspace URI
        if let Some(provider) = self.imp().completion_provider.borrow().clone() {
            let provider = provider.imp();
            provider.uri.replace(Some(uri.to_owned()));
            provider
                .workspace_uri
                .replace(Some(workspace_uri.to_owned()));
        }
    }

    /// Gets the file path from a FileInfo, if it exists, or None.
    pub fn file_path(&self) -> Option<String> {
        if let Some(ref file) = *self.imp().file.borrow()
            && let Some(path) = file.location().unwrap().path()
        {
            Some(path.to_str().unwrap().to_string().clone())
        } else {
            None
        }
    }

    /// Sets `file_path` and `file` without loading contents
    pub fn set_file(&self, path: &str) {
        let file = sourceview5::File::builder()
            .location(&gio::File::for_path(path))
            .build();
        self.imp().file.replace(Some(file));
    }

    /// Checks if file associated with buffer is externally modified
    pub fn is_externally_modified(&self) -> bool {
        if let Some(ref file) = *self.imp().file.borrow() {
            file.check_file_on_disk();
            file.is_externally_modified()
        } else {
            false
        }
    }
}

mod actions;
mod annotations;
mod hacks;
mod io;
mod lsp_ui;
