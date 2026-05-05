use std::cell::RefCell;

use async_lsp::lsp_types::DiagnosticSeverity;
use gtk::{CssProvider, SearchBar, SearchEntry, Stack};
use sourceview5::StyleSchemeManager;

use crate::{
    config::Config,
    ui::{buffer::Buffer, sidebar::Sidebar, terminal::TerminalView, welcome::WelcomeScreen},
    utils,
};

use super::*;

#[derive(Debug, Default, Properties, CompositeTemplate)]
#[template(file = "resources/ui/window.blp")]
#[properties(wrapper_type = super::EditorWindow)]
pub struct EditorWindow {
    #[template_child]
    pub tab_view: TemplateChild<TabView>,
    #[template_child]
    pub split_view: TemplateChild<OverlaySplitView>,
    #[template_child]
    pub window_title: TemplateChild<WindowTitle>,
    #[template_child]
    pub startpage: TemplateChild<WelcomeScreen>,
    #[template_child]
    pub search_bar: TemplateChild<SearchBar>,
    #[template_child]
    pub search_entry: TemplateChild<SearchEntry>,
    #[template_child]
    pub terminal_view: TemplateChild<TerminalView>,
    #[template_child]
    pub sidebar: TemplateChild<Sidebar>,
    #[template_child]
    pub stack: TemplateChild<Stack>,
    /* Status bar labels */
    #[template_child]
    pub status_lsp_load: TemplateChild<gtk::Box>,
    #[template_child]
    pub status_files: TemplateChild<gtk::Box>,
    /* Properties */
    #[property(get, set)]
    pub accent_style: RefCell<CssProvider>,
    #[property(get, set)]
    pub workspace_uri: RefCell<String>,
}

impl EditorWindow {
    pub fn reload_config(&self) {
        let cfg = Config::get_or_default();
        // Set theme variant
        adw::StyleManager::default().set_color_scheme(match cfg.ui.theme_variant {
            crate::config::ThemeVariant::System => adw::ColorScheme::Default,
            crate::config::ThemeVariant::Light => adw::ColorScheme::ForceLight,
            crate::config::ThemeVariant::Dark => adw::ColorScheme::ForceDark,
        });
        // Set theme
        if let Some(ref scheme) = StyleSchemeManager::new().scheme(&cfg.ui.theme) {
            let scheme = if adw::StyleManager::default().is_dark() {
                &utils::scheme::dark(scheme)
            } else {
                scheme
            };
            self.accent_style
                .borrow()
                .load_from_string(&utils::recolor::generate_css(scheme.clone()));
        }
        self.tab_view.pages().iter::<TabPage>().for_each(|page| {
            if let Some(buffer) = page.unwrap().child().downcast_ref::<Buffer>() {
                buffer.configure();
            }
        });
    }

    pub fn new_tab(&self, path: Option<&str>) -> TabPage {
        // Get rid of welcome page
        self.stack.set_visible_child_name("tabs");
        // Buffer
        let buffer = sourceview5::Buffer::new(None);
        buffer.set_highlight_syntax(true);
        // Create buffer UI and page
        let buffer_ui = Buffer::with_source(buffer.clone());
        let page = self.tab_view.add_page(&buffer_ui, None);
        buffer_ui.set_file_path(path.unwrap_or_default());
        buffer_ui.update_uri(
            path.unwrap_or_default(),
            &self.workspace_uri.borrow().clone(),
        );
        buffer_ui.configure();
        // Files
        if let Some(path) = path {
            let file = gio::File::for_path(path);
            let file = sourceview5::File::builder().location(&file).build();
            let loader = sourceview5::FileLoader::new(&buffer, &file);
            loader.load_async(glib::Priority::default(), gio::Cancellable::NONE, |_| {});
            // Set syntax highlighting
            if let Some(ref language) =
                sourceview5::LanguageManager::new().guess_language(Some(&path), None)
            {
                buffer.set_language(Some(language));
            }
        }
        // LSP servers
        if let Some(path) = path {
            lsp::file::open(path, self.workspace_uri.borrow().as_str());
        }
        // Add tab
        page.set_title(&buffer_ui.short_name());
        if page.is_selected() {
            self.window_title.set_title(&page.title());
            buffer_ui.set_status(&self.workspace_uri.borrow());
        }
        // Connect modified
        buffer.connect_modified_changed(clone!(
            #[strong]
            page,
            #[strong]
            buffer_ui,
            #[strong(rename_to=window_title)]
            self.window_title,
            move |_| {
                // Set title
                page.set_title(&buffer_ui.short_name());
                if page.is_selected() {
                    window_title.set_title(&page.title());
                }
            }
        ));
        buffer.connect_changed(clone!(
            #[weak(rename_to=this)]
            self,
            move |buffer| {
                if !buffer.is_loading() {
                    // Update LSP
                    if !buffer_ui.file_path().is_empty() && !this.workspace_uri.borrow().is_empty()
                    {
                        lsp::file::change(
                            &buffer_ui.file_path(),
                            &this.workspace_uri.borrow(),
                            Some(
                                buffer
                                    .text(&buffer.bounds().0, &buffer.bounds().1, false)
                                    .to_string(),
                            ),
                        );
                    }
                }
            }
        ));
        // Return page
        page
    }

    /// Set up displaying diagnostics for files
    fn setup_diagnostics(&self) {
        let rx = lsp::listen_diagnostics();
        let tab_view = self.tab_view.clone();
        glib::spawn_future_local(async move {
            while let Ok(diagnostics) = rx.recv().await {
                let uri = diagnostics.uri.path();
                // Look for a buffer with corresponding uri
                tab_view.pages().iter::<TabPage>().for_each(|page| {
                    if let Some(buffer) = page.unwrap().child().downcast_ref::<Buffer>()
                        && buffer.file_path() == uri
                    {
                        buffer.clear_diagnostics();
                        let mut start_lines = vec![];
                        diagnostics.diagnostics.iter().for_each(|diag| {
                            buffer.highlight_diagnostic(
                                &diag.range,
                                diag.message.clone(),
                                diag.severity.unwrap_or(DiagnosticSeverity::INFORMATION),
                                !start_lines.contains(&diag.range.start.line),
                            );
                            if !start_lines.contains(&diag.range.start.line) {
                                start_lines.push(diag.range.start.line);
                            }
                        });
                    }
                });
            }
        });
    }
}

impl From<EditorWindow> for gtk::Window {
    fn from(val: EditorWindow) -> Self {
        (&val as &dyn Any)
            .downcast_ref::<gtk::Window>()
            .unwrap()
            .clone()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for EditorWindow {
    const NAME: &'static str = "EditorWindow";
    type Type = super::EditorWindow;
    type ParentType = adw::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        TerminalView::static_type();
        WelcomeScreen::static_type();
        libpanel::Dock::static_type();
        Self::bind_template(klass);
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for EditorWindow {
    fn constructed(&self) {
        self.parent_constructed();
        self.obj().setup_actions();
        self.obj().setup_callbacks();
        self.obj()
            .update_folder(std::env::home_dir().unwrap().to_str().unwrap());
        self.reload_config();
        self.setup_diagnostics();
        // Style
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("Could not connect to a display."),
            &self.accent_style.borrow().clone(),
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        // Set up LSP loading spinner
        let listener = lsp::loading::listen();
        let lsp_load = self.status_lsp_load.clone();
        glib::spawn_future_local(async move {
            while listener.recv().await.is_ok() {
                lsp_load.set_visible(lsp::loading::is_loading());
            }
        });
    }
}
impl WidgetImpl for EditorWindow {}
impl WindowImpl for EditorWindow {}
impl ApplicationWindowImpl for EditorWindow {}
impl AdwApplicationWindowImpl for EditorWindow {}
