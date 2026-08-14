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
}
