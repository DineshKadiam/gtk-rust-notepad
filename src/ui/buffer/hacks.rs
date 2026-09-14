use sourceview5::MarkAttributes;

// Shitty GTK hacks that I hate
use super::*;

impl Buffer {
    /// Custom automatic line scrolling that overrides gtktextview's default.
    /// god, when will this suffering end.
    /// Basically I read through gtktextview's source code (hell) and realised that it determines if you should scroll when
    /// content is modified (e.g. pasting in new content, adding new lines...) which is awesome! but. holy fuck did they not realise
    /// the visual accessibility issue of only scrolling the baree minimum amount. Like it's just so the glyphs can be in view, nothing
    /// with margins at all. That fucking sucks. I *would* open an issue and even gladly patch this but the maintainers will probably tell
    /// me to fuck off again (as with every other fucking GTK hack in this program). God I fucking hate this.
    pub fn override_scrolling(buffer: sourceview5::Buffer, source_view: sourceview5::View) {
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

    /// Modifies the buffer text in place, and then attempts to both set the cursor back to its
    /// original position and scroll to the original position. Good for reloading files, or formatting
    /// files.
    pub fn set_buffer_text(&self, text: &str, reversible: bool) {
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
        let mut bounds = buffer.bounds();
        if reversible {
            buffer.begin_user_action();
        } else {
            buffer.begin_irreversible_action();
        }
        buffer.delete(&mut bounds.0, &mut bounds.1);
        buffer.insert(&mut buffer.start_iter(), text.trim_end_matches("\n"));
        if reversible {
            buffer.end_user_action();
        } else {
            buffer.end_irreversible_action();
        }
        buffer.set_modified(false);
        // Move cursor back if that position still exists
        let mut start = buffer.start_iter();
        if start.forward_chars(cursor) {
            buffer.place_cursor(&start);
        }
        self.imp().source_view.set_buffer(Some(&buffer));
        // Update diagnostics
        let diagnostics = self.imp().diagnostics.borrow().clone();
        self.clear_diagnostics();
        gtk::glib::timeout_add_local(
            std::time::Duration::from_millis(100),
            clone!(
                #[strong(rename_to=this)]
                self,
                move || {
                    if this
                        .get_sourceview_buffer()
                        .tag_table()
                        .lookup("diag_error")
                        .is_none()
                    {
                        for diag in &diagnostics {
                            this.highlight_diagnostic(
                                &diag.range,
                                diag.message.clone(),
                                diag.severity.unwrap_or(DiagnosticSeverity::INFORMATION),
                            );
                        }
                    }
                    ControlFlow::Break
                }
            ),
        );
        // Scoll back FIXME: cursed gtk stuff :(
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

    // Unfuck gutters (sidebar things that have icons)
    pub fn unfuck_gutters(&self) {
        let gutter = sourceview5::prelude::ViewExt::gutter(
            &self.imp().source_view.clone(),
            gtk::TextWindowType::Left,
        );
        gutter.reorder(
            &gutter
                .last_child()
                .and_downcast::<sourceview5::GutterRenderer>()
                .unwrap(),
            -100,
        );
        self.imp().source_view.set_mark_attributes(
            "info",
            &MarkAttributes::builder()
                .icon_name("dialog-information-symbolic")
                .build(),
            3,
        );
        self.imp().source_view.set_mark_attributes(
            "warn",
            &MarkAttributes::builder()
                .icon_name("dialog-warning-symbolic")
                .build(),
            2,
        );
        self.imp().source_view.set_mark_attributes(
            "error",
            &MarkAttributes::builder()
                .icon_name("dialog-error-symbolic")
                .build(),
            1,
        );
    }

    /// Auto show map on mouse movement
    pub fn auto_map(&self) {
        let motion_controller = EventControllerMotion::new();
        let coords = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
        motion_controller.connect_motion(clone!(
            #[strong(rename_to=revealer)]
            self.imp().map_revealer,
            #[strong]
            coords,
            #[weak(rename_to=this)]
            self.imp(),
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
            self.imp().map_revealer,
            #[weak(rename_to=this)]
            self.imp(),
            move |_controller| {
                if !*this.pin_mmap.borrow() {
                    revealer.set_reveal_child(false);
                }
            }
        ));
        self.add_controller(motion_controller);
    }
}

pub mod text_view_child {
    use std::ffi::{c_int, c_void};

    use gtk::{ffi::*, glib::ffi::*, prelude::*, Widget};

    #[repr(C)]
    pub struct _GtkTextViewChild {
        pub parent_instance: GtkWidget,
        pub window_type: GtkTextWindowType,
        pub overlays: GQueue,
        pub xoffset: c_int,
        pub yoffset: c_int,
        pub child: *mut GtkWidget,
    }

    #[repr(C)]
    pub struct Overlay {
        pub link: GList,
        pub widget: *mut GtkWidget,
        pub x: c_int,
        pub y: c_int,
    }

    pub type GtkTextViewChild = _GtkTextViewChild;

    pub fn remove_overlay(widget: Widget, overlay_child: Widget) {
        unsafe {
            let wah = widget.as_ptr().cast::<GtkTextViewChild>();
            let mut iter = (*wah).overlays.head;
            while !iter.is_null() {
                let data = iter.read().data.cast::<Overlay>();
                if (*data).widget == overlay_child.as_ptr() {
                    gtk::ffi::gtk_widget_unparent((*data).widget);
                    gtk::glib::ffi::g_queue_unlink(&mut (*wah).overlays, iter);
                    gtk::glib::ffi::g_free(data as *mut c_void);
                    break;
                }
                iter = (*iter).next;
            }
        }
    }
}
