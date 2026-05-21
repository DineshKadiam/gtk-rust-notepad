use adw::AlertDialog;
use adw::ResponseAppearance;
use adw::prelude::*;
use adw::subclass::prelude::*;
use async_lsp::lsp_types::DiagnosticSeverity;
use async_lsp::lsp_types::Range;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::TextSearchFlags;
use gtk::gio;
use gtk::gio::Icon;
use gtk::glib;
use gtk::pango::Underline;
use sourceview5::Annotation;
use sourceview5::AnnotationProvider;
use sourceview5::BackgroundPatternType;
use sourceview5::FileSaverFlags;
use sourceview5::StyleSchemeManager;
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::io::Error;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use gtk::{
    EventControllerMotion, Revealer,
    glib::{ControlFlow, SourceId, clone},
};

use crate::config::Config;
use crate::lsp;
use crate::ui::completion::provider::LspCompletionProvider;
use crate::ui::window::EditorWindow;
use crate::utils;

mod imp {
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

        // Variables
        pub completion_provider: RefCell<Option<LspCompletionProvider>>,
        pub file: RefCell<Option<sourceview5::File>>,
        pub mmap_timer: RefCell<Option<SourceId>>,
        pub pin_mmap: RefCell<bool>,
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
            let coords = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
            motion_controller.connect_motion(clone!(
                #[strong(rename_to=revealer)]
                self.map_revealer,
                #[strong]
                coords,
                #[weak(rename_to=this)]
                self,
                move |_controller, x, y| {
                    if !*this.pin_mmap.borrow()
                        && coords.0.swap(x as usize, Ordering::Relaxed) != x as usize
                        && coords.1.swap(y as usize, Ordering::Relaxed) != y as usize
                        && let Some(old) =
                            this.mmap_timer
                                .borrow_mut()
                                .replace(glib::timeout_add_seconds_local(
                                    1,
                                    clone!(
                                        #[strong]
                                        revealer,
                                        #[strong]
                                        this,
                                        move || {
                                            revealer.set_reveal_child(false);
                                            this.mmap_timer.take();
                                            ControlFlow::Break
                                        }
                                    ),
                                ))
                    {
                        revealer.set_reveal_child(true);
                        old.remove();
                    }
                }
            ));
            motion_controller.connect_leave(clone!(
                #[strong(rename_to=revealer)]
                self.map_revealer,
                #[weak(rename_to=this)]
                self,
                move |_controller| {
                    if !*this.pin_mmap.borrow() {
                        revealer.set_reveal_child(false);
                    }
                }
            ));
            self.obj().add_controller(motion_controller);
            // Set up source view
            self.source_view
                .annotations()
                .add_provider(&self.diagnostic_provider.borrow().clone());
            // Set up completion
            self.completion_provider
                .replace(Some(LspCompletionProvider::new()));
            self.source_view
                .completion()
                .add_provider(&self.completion_provider.borrow().clone().unwrap());
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
        glib::Object::builder::<Self>().build()
    }

    /// Creates a buffer ui/tab thingy with a SourceView Buffer object
    pub fn with_source(buf: sourceview5::Buffer) -> Self {
        let widget = glib::Object::builder::<Self>().build();
        widget.imp().source_view.set_buffer(Some(&buf));
        widget
    }

    /// Gets a sanitized, readable name for the buffer -- its path relative to the workspace directory.
    pub fn clean_name(&self, workspace_dir: &str) -> String {
        // TODO we can actually store this, and eliminate the need to borrow a workspace dir EVERY time
        // store in Buffer::name()
        if self.file_path().is_empty() {
            "[scratch]".to_string()
        } else {
            self.file_path()
                .replace(workspace_dir, "")
                .trim_start_matches("/")
                .to_string()
        }
    }

    /// Gets the short name of the buffer, plus a string if it was modified.
    pub fn tab_name(&self) -> String {
        (if !self.file_path().is_empty() {
            utils::basename(&self.file_path()).to_string()
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

    /// Clear diagnostics
    pub fn clear_diagnostics(&self) {
        self.imp().diagnostic_provider.borrow().remove_all();
        // Clear tags
        let buffer = self.get_sourceview_buffer();
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
                    Some(&message.replace("\n", " / ")),
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
        let buffer = self.get_sourceview_buffer();
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
        if let Some(ref scheme) = StyleSchemeManager::new().scheme(&config.ui.theme) {
            if adw::StyleManager::default().is_dark() {
                buffer.set_style_scheme(Some(&utils::scheme::dark(scheme)));
            } else {
                buffer.set_style_scheme(Some(scheme));
            }
        }
        self.imp().map_revealer.set_visible(config.ui.show_minimap);
        if config.ui.word_wrap {
            self.imp().pin_mmap.replace(true);
            self.imp()
                .map_revealer
                .set_reveal_child(config.ui.show_minimap);
        }
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
        if let Some(provider) = self.imp().completion_provider.borrow().clone() {
            let provider = provider.imp();
            provider.uri.replace(Some(uri.to_owned()));
            provider
                .workspace_uri
                .replace(Some(workspace_uri.to_owned()));
        }
    }

    /// Loads a file from a path. The loading process is spawned in another thread.
    pub fn load_file(&self, path: &str) {
        let buffer = self.get_sourceview_buffer();
        let file = sourceview5::File::builder()
            .location(&gio::File::for_path(path))
            .build();
        let loader = sourceview5::FileLoader::new(&buffer, &file);
        loader.load_async(glib::Priority::default(), gio::Cancellable::NONE, |_| {});
        // Set file instance
        self.imp().file.replace(Some(file));
        self.set_file_path(path);
        // Set syntax highlighting
        if let Some(ref language) =
            sourceview5::LanguageManager::new().guess_language(Some(&path), None)
        {
            buffer.set_language(Some(language));
        }
    }

    /// Loads a file from a path, asynchronously
    pub async fn load_file_async(&self, path: &str) {
        let buffer = self.get_sourceview_buffer();
        let file = sourceview5::File::builder()
            .location(&gio::File::for_path(path))
            .build();
        let loader = sourceview5::FileLoader::new(&buffer, &file);
        // Set file instance
        self.imp().file.replace(Some(file));
        self.set_file_path(path);
        // Set syntax highlighting
        if let Some(ref language) =
            sourceview5::LanguageManager::new().guess_language(Some(&path), None)
        {
            buffer.set_language(Some(language));
        }
        // Load
        loader
            .load_future(glib::Priority::default())
            .0
            .await
            .unwrap();
    }

    /// Reloads current file
    pub fn reload_file(&self) {
        let cursor = self.get_sourceview_buffer().cursor_position();
        let path = self.imp().file_path.borrow().clone();
        glib::spawn_future_local(clone!(
            #[weak(rename_to=this)]
            self,
            async move {
                this.load_file_async(&path).await;
                // Move cursor back if that position still exists
                let buffer = this.get_sourceview_buffer();
                let mut bound = buffer.bounds().0;
                if bound.forward_chars(cursor) {
                    buffer.place_cursor(&bound);
                }
            }
        ));
    }

    /// Sets `file_path` and `file` without loading contents
    pub fn set_file(&self, path: &str) {
        let file = sourceview5::File::builder()
            .location(&gio::File::for_path(path))
            .build();
        self.set_file_path(path);
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

    /// Tries to save the file -- if there is a conflict (the file was edited externally), show an alert
    pub fn try_save(&self, window: &EditorWindow) {
        if let Some(ref file) = *self.imp().file.borrow() {
            // Checks if file is externally modified
            file.check_file_on_disk();
            if file.is_externally_modified() {
                let dialog = AlertDialog::new(
                    Some("File modified..."),
                    Some(&format!(
                        "<tt>{}</tt> has been modified by an external program. Really save?",
                        self.clean_name(&window.imp().workspace_uri.borrow().clone())
                    )),
                );
                dialog.set_body_use_markup(true);
                let gtk_window: &gtk::Window = &window.clone().into();
                dialog.add_responses(&[
                    ("cancel", "_Cancel"),
                    ("save", "_Force save"),
                    ("reload", "_Force reload"),
                ]);
                dialog.set_response_appearance("save", ResponseAppearance::Destructive);
                dialog.set_response_appearance("reload", ResponseAppearance::Destructive);
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");
                dialog.connect_response(
                    Some("save"),
                    clone!(
                        #[weak(rename_to=buffer)]
                        self,
                        #[strong]
                        window,
                        move |_, _| {
                            buffer.save(&window);
                        }
                    ),
                );
                dialog.connect_response(
                    Some("reload"),
                    clone!(
                        #[weak(rename_to=buffer)]
                        self,
                        move |_, _| {
                            buffer.reload_file();
                        }
                    ),
                );
                adw::prelude::AdwDialogExt::present(&dialog, Some(gtk_window));
            } else {
                self.save(window);
            }
        }
    }

    /// Asynchronously writes the buffer to `Buffer::file_path` and updates both the associated window and LSP.
    pub fn save(&self, window: &EditorWindow) {
        let workspace_uri = window.imp().workspace_uri.borrow().clone();
        // <- TODO format before saving right here
        glib::spawn_future_local(clone!(
            #[weak]
            window,
            #[weak(rename_to=buffer)]
            self,
            async move {
                let selected_page = window.imp().tab_view.selected_page();
                // Save, and then...
                buffer.write().await;
                // Tell LSP file was modified
                lsp::file::change(&buffer.file_path(), &workspace_uri, None);
                lsp::file::save(&buffer.file_path(), &workspace_uri);
                // Format and reload buffer
                lsp::format::format(&buffer.file_path(), &workspace_uri);
                buffer.reload_file();
                // Set window/tab title
                if let Some(page) = selected_page {
                    page.set_title(&buffer.tab_name());
                    if page.is_selected() {
                        // TODO clearly i should be able to simplify this right
                        window.imp().window_title.set_title(&page.title());
                        buffer.set_status(&window.imp().workspace_uri.borrow());
                    }
                }
            }
        ));
    }

    /// Force writes buffer to a file.
    pub async fn write(&self) {
        if let Some(ref file) = self.imp().file.borrow().clone() {
            let save = sourceview5::FileSaver::new(
                self.imp()
                    .source_view
                    .buffer()
                    .downcast_ref::<sourceview5::Buffer>()
                    .unwrap(),
                file,
            );
            save.set_flags(FileSaverFlags::IGNORE_MODIFICATION_TIME);
            save.save_future(glib::Priority::default()).0.await.unwrap();
        }
    }
}
