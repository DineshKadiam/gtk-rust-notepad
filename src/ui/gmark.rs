use gtk::Align;
// TODO: this should be its own rust crate, actually...
use gtk::Orientation;
use gtk::WrapMode;
use gtk::prelude::*;
use markdown::mdast::{
    Code, Heading, InlineCode, Link, Node, Paragraph, Strong, Text, ThematicBreak,
};
use sourceview5::prelude::BufferExt;
use vte4::BoxExt;

use crate::config::Config;
use crate::utils;

/// Parses Markdown to GTK objects.
pub fn parse(input: &str) -> gtk::Box {
    let out = gtk::Box::new(Orientation::Vertical, 16);
    if let Ok(Node::Root(mdast)) = markdown::to_mdast(&input, &markdown::ParseOptions::gfm()) {
        for node in mdast.children {
            out.append(&parse_node(node));
        }
    }
    out
}

fn parse_node(node: Node) -> gtk::Widget {
    let out = match node {
        // Regular text
        Node::Text(Text { value, position: _ }) => {
            gtk::Label::new(Some(&value)).upcast::<gtk::Widget>()
        }

        // Paragraph
        Node::Paragraph(Paragraph {
            children,
            position: _,
        }) => {
            let out = gtk::Box::new(Orientation::Vertical, 16);
            for child in children {
                out.append(&parse_node(child));
            }
            out.upcast::<gtk::Widget>()
        }

        // Spacer
        Node::ThematicBreak(ThematicBreak { position: _ }) => {
            gtk::Separator::new(Orientation::Horizontal).upcast::<gtk::Widget>()
        }

        // Bolded text
        Node::Strong(Strong {
            children,
            position: _,
        }) => {
            let out = adw::WrapBox::new();
            for child in children {
                out.append(&parse_node(child));
            }
            out.upcast::<gtk::Widget>()
        }

        // Inline code
        Node::InlineCode(InlineCode { value, position: _ }) => gtk::Label::builder()
            .label(&value)
            .css_classes(["monospace"])
            .build()
            .upcast::<gtk::Widget>(),

        // Code
        Node::Code(Code {
            value,
            position: _,
            lang,
            meta: _,
        }) => {
            let view = sourceview5::View::new();
            let buffer = view.buffer().downcast::<sourceview5::Buffer>().unwrap();
            view.add_css_class("monospace");
            view.set_editable(false);
            view.set_wrap_mode(WrapMode::Word);
            buffer.set_text(&value);
            // Set language
            if let Some(lang) = lang
                && let Some(ref language) = sourceview5::LanguageManager::new().language(&lang)
            {
                buffer.set_language(Some(language));
            }
            // Set theme (Kyanite)
            let config = Config::get_or_default();
            if let Some(ref scheme) =
                sourceview5::StyleSchemeManager::new().scheme(&config.ui.theme)
            {
                if adw::StyleManager::default().is_dark() {
                    buffer.set_style_scheme(Some(&utils::scheme::dark(scheme)));
                } else {
                    buffer.set_style_scheme(Some(scheme));
                }
            }
            view.upcast::<gtk::Widget>()
        }

        // Heading
        Node::Heading(Heading {
            children,
            position: _,
            depth,
        }) => {
            let out = adw::WrapBox::new();
            out.add_css_class(&format!("title-{depth}"));
            for child in children {
                out.append(&parse_node(child));
            }
            out.upcast::<gtk::Widget>()
        }

        // Links
        Node::Link(Link {
            children: _,
            position: _,
            url,
            title,
        }) => gtk::LinkButton::builder()
            .label(&title.unwrap_or(url.clone()))
            .uri(url)
            .build()
            .upcast::<gtk::Widget>(),
        _ => adw::Bin::new().upcast::<gtk::Widget>(),
    };
    if out.downcast_ref::<gtk::Label>().is_some() || out.downcast_ref::<gtk::LinkButton>().is_some()
    {
        out.set_halign(Align::Start);
    }
    out
}
