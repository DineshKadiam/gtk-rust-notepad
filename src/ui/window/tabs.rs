use std::sync::RwLock;

use gtk::{
    gio::{SimpleAction, SimpleActionGroup},
    prelude::*,
    subclass::prelude::*,
};

use crate::ui::window::EditorWindow;

macro_rules! create_action {
    ($ident: ident, $name: expr, $f: expr) => {{
        let action = SimpleAction::new($name, None);
        let $ident = $ident.clone();
        action.connect_activate(move |_, _| $f());
        action
    }};
}

/// Connect tab context menus
/// Libadwaita is a hacky undocumented mess and you all should be ashamed of yourselves.
/// Which is to say, hack below. There's no way to do this in a memory-safe way.
/// OH MY GOD THIS IS THE OFFICIAL WAY TO DO IT >:(
/// -- stolen from pages
pub fn init_tab_menus(window: &EditorWindow) {
    // Yep! Global static. This one is safe, I think. It stores the index of the tab that's been right-clicked.
    // In the worst case, it doesn't return a null pointer (take THAT c libadwaita progrms!!) but an integer. Nice!
    static MENU_TAB_STATE: RwLock<i32> = RwLock::new(0);
    // Create a group called "tab" and new actions that pin/unpin/close/etc tabs given MENU_TAB_STATE
    let group = SimpleActionGroup::new();
    // Pin/unpin
    let action_pin = create_action!(window, "pin", || {
        let page = window
            .imp()
            .tab_view
            .nth_page(*MENU_TAB_STATE.read().unwrap());
        window.imp().tab_view.set_page_pinned(&page, true);
        *MENU_TAB_STATE.write().unwrap() = 0;
    });
    let action_unpin = create_action!(window, "unpin", || {
        let page = window
            .imp()
            .tab_view
            .nth_page(*MENU_TAB_STATE.read().unwrap());
        window.imp().tab_view.set_page_pinned(&page, false);
        *MENU_TAB_STATE.write().unwrap() = 0;
    });
    group.add_action(&action_pin);
    group.add_action(&action_unpin);
    // Other actions
    group.add_action(&create_action!(window, "close", || {
        let page = window
            .imp()
            .tab_view
            .nth_page(*MENU_TAB_STATE.read().unwrap());
        window.imp().tab_view.close_page(&page);
        *MENU_TAB_STATE.write().unwrap() = 0;
    }));
    group.add_action(&create_action!(window, "close_other", || {
        let page = window
            .imp()
            .tab_view
            .nth_page(*MENU_TAB_STATE.read().unwrap());
        window.imp().tab_view.close_other_pages(&page);
        *MENU_TAB_STATE.write().unwrap() = 0;
    }));
    group.add_action(&create_action!(window, "close_left", || {
        let page = window
            .imp()
            .tab_view
            .nth_page(*MENU_TAB_STATE.read().unwrap());
        window.imp().tab_view.close_pages_before(&page);
        *MENU_TAB_STATE.write().unwrap() = 0;
    }));
    group.add_action(&create_action!(window, "move_left", || {
        let page = window
            .imp()
            .tab_view
            .nth_page(*MENU_TAB_STATE.read().unwrap());
        window.imp().tab_view.reorder_backward(&page);
        *MENU_TAB_STATE.write().unwrap() = 0;
    }));
    group.add_action(&create_action!(window, "move_right", || {
        let page = window
            .imp()
            .tab_view
            .nth_page(*MENU_TAB_STATE.read().unwrap());
        window.imp().tab_view.reorder_forward(&page);
        *MENU_TAB_STATE.write().unwrap() = 0;
    }));
    group.add_action(&create_action!(window, "move_new", || {
        let page = window
            .imp()
            .tab_view
            .nth_page(*MENU_TAB_STATE.read().unwrap());
        let app = window
            .application()
            .unwrap()
            .downcast_ref::<adw::Application>()
            .unwrap()
            .clone();
        let new_window = EditorWindow::new(&app);
        // Get rid of welcome page
        new_window
            .imp()
            .tab_view
            .close_page(&new_window.imp().tab_view.selected_page().unwrap());
        new_window.present();
        window.imp().tab_view.transfer_page(
            &page,
            new_window
                .imp()
                .tab_view
                .downcast_ref::<adw::TabView>()
                .unwrap(),
            0,
        );
        *MENU_TAB_STATE.write().unwrap() = 0;
    }));
    window.insert_action_group("tab_context", Some(&group));
    // Now, whenever someone right-clicks on a TabView, display this menu and set MENU_TAB_STATE
    window
        .imp()
        .tab_view
        .connect_setup_menu(move |tab_view, page| {
            if let Some(page) = page {
                if page.is_pinned() {
                    action_unpin.set_enabled(true);
                    action_pin.set_enabled(false);
                } else {
                    action_unpin.set_enabled(false);
                    action_pin.set_enabled(true);
                }
                *MENU_TAB_STATE.write().unwrap() = tab_view.page_position(page);
            } else {
                *MENU_TAB_STATE.write().unwrap() = 0;
            }
        });
}
