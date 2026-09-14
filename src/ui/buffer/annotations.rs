use std::rc::Rc;

use gtk::{Button, Image, Label, Orientation};

use super::*;
use crate::ui::dialog::hover::HoverDialog;

const MARGIN: i32 = 16;

#[derive(Debug, Clone)]
pub struct Annotation {
    widget: gtk::Widget,
    line: i32,
    messages: Rc<RefCell<Vec<String>>>,
}

impl Annotation {
    pub fn new(message: &str, line: i32) -> Self {
        let messages = Rc::new(RefCell::new(vec![message.to_string()]));
        let widget = {
            let message_line = message.split("\n").next().unwrap();
            let messages = messages.clone();
            let button = Button::new();
            let contents = gtk::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(8)
                .build();
            contents.append(&Label::new(Some(&if message_line.len() < 48 {
                message_line.to_string()
            } else {
                format!("{}…", &message_line[..48])
            })));
            contents.append(
                &Image::builder()
                    .icon_name("view-fullscreen-symbolic")
                    .build(),
            );
            button.set_child(Some(&contents));
            button.set_css_classes(&["inlay", "pill"]);
            button.set_cursor(Some(&gtk::gdk::Cursor::from_name("default", None).unwrap()));
            button.connect_clicked(move |btn| {
                let messages = messages.borrow();
                if messages.len() == 1 {
                    HoverDialog::new(&messages[0]).present(btn);
                } else {
                    HoverDialog::new_multiple(&messages).present(btn);
                }
            });
            button
        }
        .upcast();
        Annotation {
            widget,
            line,
            messages,
        }
    }

    pub fn add_css_class(&self, class: &str) {
        if !(self.widget.has_css_class("error") && class == "warning")
            && !self.widget.has_css_class(class)
        {
            self.widget.add_css_class(class)
        }
    }

    pub fn update_label(&self) {
        if let Some(contents) = self.widget.first_child().and_downcast::<gtk::Box>()
            && let Some(label) = contents.first_child().and_downcast::<gtk::Label>()
        {
            label.set_label(&format!("{} messages", self.messages.borrow().len()));
        }
    }

    pub fn push(&self, msg: &str) {
        self.messages.borrow_mut().push(msg.to_string());
    }
}

impl super::Buffer {
    pub fn new_annotation(&self, message: &str, line: i32) -> Result<Annotation, ()> {
        let mut annotations = self.imp().annotations.borrow_mut();
        if let Some(annotation) = annotations.iter_mut().find(|a| a.line == line) {
            annotation.push(message);
            annotation.update_label();
            Ok(annotation.clone())
        } else {
            drop(annotations);
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
                Ok(annotation.clone())
            } else {
                Err(())
            }
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
                this.update_annotations();
            }
        ));
    }
    
    pub fn update_annotations(&self) {
        // TODO use TextMark to mark a position and just update on that
        let annotations = self.imp().annotations.borrow().clone();
        let buffer = self.get_sourceview_buffer();
        for annotation in annotations {
            if let Some(mut iter) = buffer.iter_at_line(annotation.line) {
                iter.forward_to_line_end();
                let location = self.imp().source_view.iter_location(&iter);
                self.imp().source_view.move_overlay(
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
}

