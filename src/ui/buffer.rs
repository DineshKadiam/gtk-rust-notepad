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
use gtk::gio::Cancellable;
use gtk::gio::FileQueryInfoFlags;
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
use std::fs;
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
use crate::ui::sidebar;
use crate::ui::window::EditorWindow;
use crate::utils;

mod imp {
    use gtk::pango::{FontDescription, FontMap};

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

            // Custom SourceMap fonts
            self.source_map.set_font_map(None::<&FontMap>);
            self.source_map
                .set_font_desc(Some(&FontDescription::from_string("KyaniteBlocks")));

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
            // Scroll to the bottom when new lines are added; with a margin (unlike by default)
            super::Buffer::override_scrolling(
                self.obj().get_sourceview_buffer(),
                self.source_view.clone(),
            );
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
    /// TODO make this an alias to load_file_async
    pub fn load_file(&self, path: &str) {
        let buffer = self.get_sourceview_buffer();
        let gio_file = gio::File::for_path(path);
        let file = sourceview5::File::builder().location(&gio_file).build();
        let info = gio_file
            .query_info("standard::*", FileQueryInfoFlags::NONE, Cancellable::NONE)
            .unwrap();
        let loader = sourceview5::FileLoader::new(&buffer, &file);
        loader.load_async(glib::Priority::default(), gio::Cancellable::NONE, |_| {});
        // Set file instance
        self.imp().file.replace(Some(file));
        // Set syntax highlighting
        if let Some(ref language) = sourceview5::LanguageManager::new()
            .guess_language(Some(&path), info.content_type().as_deref())
        {
            buffer.set_language(Some(language));
        }
    }

    /// Loads a file from a path, asynchronously
    pub async fn load_file_async(&self, path: &str) {
        let buffer = self.get_sourceview_buffer();
        let gio_file = gio::File::for_path(path);
        let file = sourceview5::File::builder().location(&gio_file).build();
        let info = gio_file
            .query_info("standard::*", FileQueryInfoFlags::NONE, Cancellable::NONE)
            .unwrap();
        let loader = sourceview5::FileLoader::new(&buffer, &file);
        // Set file instance
        self.imp().file.replace(Some(file));
        // Set syntax highlighting
        if let Some(ref language) = sourceview5::LanguageManager::new()
            .guess_language(Some(&path), info.content_type().as_deref())
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

    /// Reloads current file
    pub fn reload_file(&self) {
        if let Some(path) = self.file_path() {
            self.set_buffer_text(&fs::read_to_string(&path).unwrap());
            self.set_file(&path);
        }
    }

    /// Modifies the buffer text in place, and then attempts to both set the cursor back to its
    /// original position and scroll to the original position. Good for reloading files, or formatting
    /// files.
    pub fn set_buffer_text(&self, text: &str) {
        // Save cursor/scroll positions
        let cursor = self.get_sourceview_buffer().cursor_position();
        let scroll_window = self
            .imp()
            .source_view
            .parent()
            .and_downcast::<gtk::ScrolledWindow>()
            .unwrap();
        let (scroll_y, scroll_x) = (
            scroll_window.vadjustment().value(),
            scroll_window.hadjustment().value(),
        );
        // Get buffer
        let buffer = self.get_sourceview_buffer();
        // Reload manually, blocking
        buffer.set_text(text);
        buffer.set_modified(false);
        // Move cursor back if that position still exists
        let mut start = buffer.start_iter();
        if start.forward_chars(cursor) {
            buffer.place_cursor(&start);
        }
        self.imp().source_view.set_buffer(Some(&buffer));
        // Scroll back FIXME: cursed gtk stuff :(
        // GTK4 recomputes the size of the SourceView widget as it reloads from the buffer. This cases it to scroll up
        // for an indeterminate time until it's done loading. There's no signal I can follow for when it's done, so I
        // don't know when to scroll back safely. Instead, I have to continually set the horizontal/scroll values
        // until GTK stops overriding them each time.
        // This connects to whenever the `value` attribute on a scrollbar has changed, and only disconnects it when
        // GTK hasn't overridden it. This has to use a sender/receiver pair because I can't access the handler ID from
        // inside the closure.
        //
        // This is wildly inefficient but the only way to get sane reloading (the only way to actually set buffer text
        // without GTK being annoying). This has made me genuinely consider using iced widgets instead of GTK ones, which
        // I might do in the future anyway, because I'm fed up with putting up with GTK's shit.
        macro_rules! scroll {
            ($adj: expr, $to: expr) => {
                let adj = $adj;
                let (tx, rx) = async_channel::unbounded();
                adj.set_value($to);
                let handler_id = $adj.connect_value_notify(move |adjustment| {
                    if adjustment.value() != $to && adjustment.value() != adjustment.upper() {
                        adjustment.set_value($to);
                    } else {
                        tx.send_blocking(()).unwrap();
                    }
                });
                glib::spawn_future_local(async move {
                    rx.recv().await.unwrap();
                    adj.disconnect(handler_id);
                });
            };
        }
        scroll!(scroll_window.clone().vadjustment(), scroll_y);
        scroll!(scroll_window.clone().hadjustment(), scroll_x);
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
                let path = buffer.file_path().unwrap();
                lsp::file::change(&path, &workspace_uri, None);
                lsp::file::save(&path, &workspace_uri);
                // Format and reload buffer
                if lsp::format::format(&path, &workspace_uri).await {
                    buffer.reload_file();
                }
                // Set window/tab title
                if let Some(page) = selected_page {
                    page.set_title(&buffer.tab_name());
                    if let Some(ref file) = *buffer.imp().file.borrow()
                        && let Ok(info) = file.location().unwrap().query_info(
                            "standard::*",
                            FileQueryInfoFlags::NONE,
                            Cancellable::NONE,
                        )
                    {
                        let icon = sidebar::icons::from_file_info(&info);
                        page.set_icon(Some(&icon));
                    }
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

    /// Custom automatic line scrolling that overrides gtktextview's default.
    /// god, when will this suffering end.
    /// Basically I read through gtktextview's source code (hell) and realised that it determines if you should scroll when
    /// content is modified (e.g. pasting in new content, adding new lines...) which is awesome! but. holy fuck did they not realise
    /// the visual accessibility issue of only scrolling the baree minimum amount. Like it's just so the glyphs can be in view, nothing
    /// with margins at all. That fucking sucks. I *would* open an issue and even gladly patch this but the maintainers will probably tell
    /// me to fuck off again (as with every other fucking GTK hack in this program). God I fucking hate this.
    fn override_scrolling(buffer: sourceview5::Buffer, source_view: sourceview5::View) {
        buffer.connect_cursor_moved(move |buf| {
            gtk::glib::timeout_add_local(
                std::time::Duration::from_millis(100),
                clone!(
                    #[strong]
                    source_view,
                    #[strong]
                    buf,
                    move || {
                        let mut pos = buf.start_iter();
                        pos.forward_chars(buf.cursor_position());
                        source_view.scroll_to_iter(&mut pos, 0.2, false, 0., 0.);
                        ControlFlow::Break
                    }
                ),
            );
        });
    }
}
