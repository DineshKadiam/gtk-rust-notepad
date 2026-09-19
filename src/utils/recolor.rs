use sourceview5::StyleScheme;

use crate::utils;

/// Rewrite of GNOME Builder/Text Editor's recoloring algorithms
pub fn generate_css(style_scheme: StyleScheme) -> String {
    // Don't restyle Adwaita as we already have it
    if style_scheme.id() == "Adwaita" || style_scheme.id() == "Adwaita-dark" {
        return String::new();
    }

    let is_dark = utils::scheme::is_dark(&style_scheme).unwrap_or(true);
    let alt = if is_dark { "#ffffff" } else { "#000000" };
    let mut style = "@define-color card_fg_color @window_fg_color;
      @define-color headerbar_border_color @window_fg_color;
      @define-color popover_fg_color @window_fg_color;
      @define-color dialog_fg_color @window_fg_color;
      @define-color dark_fill_bg_color @headerbar_bg_color;
      @define-color sidebar_backdrop_color mix(@sidebar_bg_color, @window_bg_color, .5);
      @define-color sidebar_fg_color @window_fg_color;
      @define-color view_fg_color @window_fg_color;"
        .to_string();

    // Window bg color (FIXME assumes text styles have a background and foreground. They better.)
    let text_style = style_scheme.style("text").unwrap();
    style += &if is_dark {
        format!(
            "@define-color window_bg_color {};\n",
            text_style.background().unwrap()
        )
    } else {
        format!(
            "@define-color window_bg_color mix({}, {}, 0.03);\n",
            text_style.background().unwrap(),
            text_style.foreground().unwrap()
        )
    };

    // Window fg color
    style += &format!(
        "@define-color window_fg_color {};",
        text_style.foreground().unwrap()
    );

    // Sidebar bg/fg color
    if let Some(line_style) = style_scheme.style("line-numbers")
        && line_style.background().unwrap() != text_style.background().unwrap()
    {
        style += &format!(
            "@define-color sidebar_bg_color mix({}, {}, 0.25);
             @define-color sidebar_fg_color mix({}, {}, 0.5);\n",
            line_style
                .background()
                .unwrap_or(text_style.background().unwrap()),
            text_style.background().unwrap(),
            line_style.foreground().unwrap(),
            alt
        );
    } else {
        style += &format!(
            "@define-color sidebar_bg_color mix({}, {}, 0.085);
             @define-color sidebar_fg_color {};\n",
            text_style.background().unwrap(),
            text_style.foreground().unwrap(),
            text_style.foreground().unwrap(),
        );
    }

    // Headerbar colors
    style += &if is_dark {
        format!(
            "@define-color headerbar_bg_color mix({}, #ffffff, 0.07);
             @define-color headerbar_fg_color {};\n",
            text_style.background().unwrap(),
            text_style.foreground().unwrap()
        )
    } else {
        format!(
            "@define-color headerbar_bg_color mix({}, #ffffff, 0.25);
             @define-color headerbar_fg_color {};\n",
            text_style.background().unwrap(),
            text_style.foreground().unwrap()
        )
    };

    // Popover colors
    style += &format!(
        "@define-color popover_bg_color alpha(mix({}, #ffffff, {}), 2.0);\n",
        text_style.background().unwrap(),
        if is_dark { 0.07 } else { 0.25 }
    );

    // Dialog colors
    style += &if is_dark {
        format!(
            "@define-color dialog_bg_color mix({}, #ffffff, 0.07);\n",
            text_style.background().unwrap()
        )
    } else {
        format!(
            "@define-color dialog_bg_color {};\n",
            text_style.background().unwrap()
        )
    };

    // View colors
    style += &if is_dark {
        format!(
            "@define-color view_bg_color mix({}, #000000, 0.1);\n",
            text_style.background().unwrap()
        )
    } else {
        format!(
            "@define-color view_bg_color mix({}, #ffffff, 0.3);\n",
            text_style.background().unwrap()
        )
    };
    style += &format!(
        "@define-color view_fg_color {};\n",
        text_style.foreground().unwrap()
    );

    // Accent colors
    if let Some(selection_style) = style_scheme.style("selection") {
        style += &format!(
            "@define-color accent_bg_color {};
             @define-color accent_fg_color mix({}, {}, 0.5);
             @define-color accent_color mix({}, {}, .1);\n",
            selection_style.background().unwrap(),
            selection_style.background().unwrap(),
            alt,
            selection_style.background().unwrap(),
            alt
        );
    }

    // suffix(???)
    style += if is_dark {
        "@define-color card_bg_color alpha(white, .08);"
    } else {
        "@define-color card_bg_color alpha(white, .8);"
    };

    style
}
