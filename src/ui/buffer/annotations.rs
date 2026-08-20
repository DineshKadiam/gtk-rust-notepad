use gtk::{Button, Image, Label, Orientation};

use crate::ui::dialog::hover::HoverDialog;

use super::*;

const MARGIN: i32 = 16;

#[derive(Debug, Clone)]
pub struct Annotation {
    widget: gtk::Widget,
    line: i32,
    messages: Vec<String>,
}

impl Annotation {
    pub fn new(message: &str, line: i32) -> Self {
        let widget = {
            let message_line = message.split("\n").next().unwrap();
            let message = message.to_string();
            let button = Button::new();
            let contents = gtk::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(8)
                .build();
            contents.append(&Label::new(Some(&message_line)));
            contents.append(
                &Image::builder()
                    .icon_name("view-fullscreen-symbolic")
                    .build(),
            );
            button.set_child(Some(&contents));
            button.set_css_classes(&["inlay", "pill"]);
            button.set_cursor(Some(&gtk::gdk::Cursor::from_name("default", None).unwrap()));
            button.connect_clicked(move |btn| {
                HoverDialog::new(&message).present(btn);
            });
            button
        }
        .upcast();
        Annotation {
            widget,
            line,
            messages: vec![message.to_string()],
        }
    }

    pub fn add_css_class(&self, class: &str) {
        if !(self.widget.has_css_class("error") && class == "warning")
            && !self.widget.has_css_class(class)
        {
            self.widget.add_css_class(class)
        }
    }
}

impl super::Buffer {
    pub fn new_annotation(&self, message: &str, line: i32) -> Result<Annotation, ()> {
        let annotation = Annotation::new(message, line);
        let buffer = self.get_sourceview_buffer();
        if let Some(mut iter) = buffer.iter_at_line(line) {
            iter.forward_to_line_end();
            let location = self.imp().source_view.iter_location(&iter);
            let mut annotations = self.imp().annotations.borrow().clone();
            self.imp().source_view.add_overlay(
                &annotation.widget,
                location.x() + MARGIN,
                location.y(),
            );
            annotations.push(annotation.clone());
            self.imp().annotations.replace(annotations);
            Ok(annotation)
        } else {
            Err(())
        }
    }

    pub fn clear_annotations(&self) {
        let annotations = self.imp().annotations.borrow().clone();
        self.imp().annotations.replace(vec![]);
        for annotation in annotations.clone() {
            if let Some(parent) = annotation.widget.parent() {
                hacks::text_view_child::remove_overlay(parent, annotation.widget.clone());
            }
            annotation.widget.unrealize();
        }
    }

    pub fn setup_annotations(&self) {
        self.get_sourceview_buffer().connect_changed(clone!(
            #[strong(rename_to=this)]
            self,
            move |_| {
                let annotations = this.imp().annotations.borrow().clone();
                let buffer = this.get_sourceview_buffer();
                for annotation in annotations {
                    if let Some(mut iter) = buffer.iter_at_line(annotation.line) {
                        iter.forward_to_line_end();
                        let location = this.imp().source_view.iter_location(&iter);
                        this.imp().source_view.move_overlay(
                            &annotation.widget,
                            location.x() + MARGIN,
                            location.y(),
                        );
                        annotation.widget.set_visible(true);
                    } else {
                        annotation.widget.set_visible(false);
                    }
                }
            }
        ));
    }
}
