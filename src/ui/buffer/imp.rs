use super::*;
use crate::ui::buffer::annotations::Annotation;

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
    #[template_child]
    pub toast_overlay: TemplateChild<adw::ToastOverlay>,

    // Variables
    pub workspace_uri: RefCell<String>,
    pub diagnostics: RefCell<Vec<Diagnostic>>,
    pub annotations: RefCell<Vec<Annotation>>,
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
        if self
            .obj()
            .pango_context()
            .font_map()
            .unwrap()
            .family("KyaniteBlocks")
            .is_some()
        {
            self.source_map.set_font_map(None::<&FontMap>);
            self.source_map
                .set_font_desc(Some(&FontDescription::from_string("KyaniteBlocks")));
        }

        // Show map on mouse motion only
        self.obj().auto_map();

        // Set up gutter mark attributes (fancy words for icons in your line number bar)
        self.obj().unfuck_gutters();
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
        // Set up annotations
        self.obj().setup_annotations();

        // Set up actions
        self.obj().setup_actions();
    }
}
impl WidgetImpl for Buffer {}
impl BoxImpl for Buffer {}
