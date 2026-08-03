use async_lsp::lsp_types::Diagnostic;

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
    pub diagnostics: RefCell<Vec<Diagnostic>>,
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

        // Set up actions
        self.obj().setup_actions();
    }
}
impl WidgetImpl for Buffer {}
impl BoxImpl for Buffer {}
