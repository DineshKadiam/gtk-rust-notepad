use adw::subclass::prelude::*;
use adw::{AlertDialog, ResponseAppearance, TabPage, prelude::*};
use gtk::Label;
use gtk::{
    gio::{Cancellable, SimpleAction},
    glib::{self, Variant, clone},
};
use sourceview5::prelude::*;

use crate::ui::dialog::rename_file;
use crate::ui::preferences::PreferencesDialog;
use crate::utils;
use crate::{
    lsp,
    ui::{buffer::Buffer, terminal::TerminalView, window::EditorWindow},
};

#[macro_export]
macro_rules! action_entry {
    ($name: expr, $type: expr, $fn: ident) => {
        ActionEntry::builder($name)
            .parameter_type($type)
            .activate(actions::$fn)
            .build()
    };
}

/// New tab
pub fn tab_new(window: &EditorWindow, _: &SimpleAction, param: Option<&Variant>) {
    let path = param.unwrap().to_string();
    window.imp().new_tab(Some(&path[1..(path.len() - 1)]));
}

/// New empty tab aka. scratch buffer
pub fn tab_scratch(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    window.imp().new_tab(None);
}

/// New terminal tab
pub fn tab_term(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    let term = TerminalView::new();
    let page = window.imp().tab_view.add_page(&term, None);
    term.setup(window.imp().workspace_uri.borrow().as_str());
    page.set_title("Terminal");
    if page.is_selected() {
        window.imp().window_title.set_title(&page.title());
    }
    window.imp().stack.set_visible_child_name("tabs");
}

/// Reloads tab
pub fn tab_reload(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(page) = window.imp().tab_view.selected_page()
        && let Some(buffer) = page.child().downcast_ref::<Buffer>()
    {
        buffer.reload_file();
    }
}

/// Closes the current tab
pub fn tab_close(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(page) = window.imp().tab_view.selected_page() {
        // LSP file close
        if let Some(buffer) = page.child().downcast_ref::<Buffer>()
            && !buffer.file_path().is_empty()
        {
            lsp::file::close(&buffer.file_path(), &window.workspace_uri());
        }
        window.imp().tab_view.close_page(&page);
    }
}

/// Toggles the sidebar
pub fn sidebar_toggle(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    let split_view = window.imp().split_view.clone();
    split_view.set_show_sidebar(!split_view.shows_sidebar());
}

/// Toggles the search bar
pub fn search_toggle(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(page) = window.imp().tab_view.selected_page()
        && let Some(buffer) = page.child().downcast_ref::<Buffer>()
    {
        let mode = !window.imp().search_bar.is_search_mode();
        window.imp().search_bar.set_search_mode(mode);
        // Focus the search entry (text widget)
        if mode {
            window.imp().search_entry.set_text("");
            window.imp().search_entry.grab_focus();
        } else {
            // Focus the current tab
            buffer.imp().source_view.grab_focus();
        }
    }
}

/// Toggles the file search bar
pub fn search_files_toggle(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    window.imp().split_view.set_show_sidebar(true);
    window.imp().sidebar.imp().file_search.grab_focus();
}

/// Creates a new window
pub fn new_window(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    let app = window
        .application()
        .unwrap()
        .downcast_ref::<adw::Application>()
        .unwrap()
        .clone();
    let new_window = EditorWindow::new(&app);
    new_window.present();
}

/// Opens a file
pub fn open_file(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    gtk::FileDialog::new().open(
        window.into(),
        Some(&Cancellable::new()),
        clone!(
            #[strong]
            window,
            move |result| {
                if let Ok(file) = result {
                    window
                        .imp()
                        .new_tab(Some(file.path().unwrap().to_str().unwrap()));
                }
            }
        ),
    );
}

/// Opens a folder
pub fn open_folder(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    gtk::FileDialog::new().select_folder(
        window.into(),
        Some(&Cancellable::new()),
        clone!(
            #[strong]
            window,
            move |result| {
                if let Ok(file) = result {
                    window.update_folder(file.path().unwrap().to_str().unwrap());
                    // close tabs + open sidebar
                    window.imp().split_view.set_show_sidebar(true);
                    for page in window.imp().tab_view.pages().iter::<TabPage>() {
                        window.imp().tab_view.close_page(&page.unwrap());
                    }
                }
            }
        ),
    );
}

/// Opens the preferences dialog
pub fn show_prefs(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    let prefs = PreferencesDialog::new();
    prefs.present(&window.clone().into());
}

/// Reloads config file and updates UI components
pub fn reload_config(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    window.imp().reload_config();
}

/// Set statusbar files segment
pub fn status_set_file(window: &EditorWindow, _: &SimpleAction, param: Option<&Variant>) {
    let param = param.unwrap();
    let file_path = glib::Variant::get::<String>(&param.child_value(0)).unwrap();
    let file_type = glib::Variant::get::<String>(&param.child_value(1)).unwrap();
    let is_shown: bool = param.child_value(2).get::<String>().unwrap() == "yes";
    utils::ui::get_child_by_name::<Label>(window.imp().status_files.get(), "status_file_path")
        .unwrap()
        .set_label(&file_path);
    utils::ui::get_child_by_name::<Label>(window.imp().status_files.get(), "status_lang")
        .unwrap()
        .set_label(&file_type);
    window.imp().status_files.set_visible(is_shown);
}

/// Open folder from file tree
pub fn tree_open_folder(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(path) = window.imp().sidebar.get_selected_folder() {
        window.update_folder(&path);
    }
}

/// Toggle a tree expander row to be expanded
pub fn tree_toggle_expand(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    window.imp().sidebar.tree_toggle_expand();
}

/// Create file from file tree
pub fn tree_new_file(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(path) = window.imp().sidebar.get_selected_folder() {
        let dialog = rename_file::FileNameDialog::new(rename_file::Kind::Create, &path);
        let window: &gtk::Window = &window.clone().into();
        adw::prelude::AdwDialogExt::present(&dialog, Some(window));
    }
}

/// Rename file/folder in file tree
pub fn tree_rename(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(path) = window.imp().sidebar.get_selected_folder() {
        // Unparent popover (if exists)
        window.imp().sidebar.imp().folder_popover.unparent();
        window.imp().sidebar.imp().file_popover.unparent();
        // Show dialog
        let dialog = rename_file::FileNameDialog::new(rename_file::Kind::Rename, &path);
        let window: &gtk::Window = &window.clone().into();
        adw::prelude::AdwDialogExt::present(&dialog, Some(window));
    }
}

/// Delete file from file tree
pub fn tree_delete_file(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(path) = window.imp().sidebar.get_selected_folder() {
        // Unparent popover (if exists)
        window.imp().sidebar.imp().folder_popover.unparent();
        window.imp().sidebar.imp().file_popover.unparent();
        // Create alert dialog
        let dialog = AlertDialog::new(
            Some("Delete file..."),
            Some(&format!(
                "Really delete <tt>{}</tt>?\n<b>It will be gone forever (a very long time).</b>",
                path.replace(std::env::home_dir().unwrap().to_str().unwrap(), "~")
            )),
        );
        dialog.set_body_use_markup(true);
        let window: &gtk::Window = &window.clone().into();
        dialog.add_responses(&[("cancel", "_Cancel"), ("confirm", "_Delete")]);
        dialog.set_response_appearance("confirm", ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.connect_response(Some("confirm"), move |_, _| {
            std::fs::remove_file(path.clone()).unwrap();
            // TODO close all tabs with the file open
        });
        adw::prelude::AdwDialogExt::present(&dialog, Some(window));
    }
}

/// Saves the current buffer
pub fn save_current_buffer(window: &EditorWindow, save_as: bool) {
    let window = window.imp();
    if let Some(page) = window.tab_view.selected_page()
        && let Some(buffer) = page.child().downcast_ref::<Buffer>()
    {
        if buffer.file_path() == "" || save_as {
            let old_path = buffer.file_path();
            let workspace_uri = window.workspace_uri.borrow().clone();
            gtk::FileDialog::new().save(
                Some::<&gtk::Window>(&window.obj().clone().into()),
                Some(&Cancellable::new()),
                clone!(
                    #[strong]
                    buffer,
                    #[strong]
                    page,
                    #[weak]
                    window,
                    move |result| {
                        if let Ok(file) = result {
                            let file_name =
                                file.path().unwrap().to_str().unwrap().to_string().clone();
                            // Set syntax highlighting
                            if let Some(ref language) = sourceview5::LanguageManager::new()
                                .guess_language(Some(&file_name), None)
                            {
                                buffer
                                    .imp()
                                    .source_view
                                    .buffer()
                                    .downcast_ref::<sourceview5::Buffer>()
                                    .unwrap()
                                    .set_language(Some(language));
                            }
                            // Set buffer path
                            buffer.set_file(&file_name);
                            // Save -- TODO: move to buffer.save() ??? buffer.save_as/new/resave() ???
                            glib::spawn_future_local(async move {
                                buffer.write().await;
                                if !old_path.is_empty() {
                                    lsp::file::close(&old_path, &workspace_uri);
                                }
                                buffer.update_uri(&file_name, &workspace_uri);
                                lsp::file::open(&file_name, &workspace_uri);
                                lsp::file::change(&file_name, &workspace_uri, None);
                                lsp::file::save(&file_name, &workspace_uri);
                                // Set window/tab title
                                page.set_title(&buffer.tab_name());
                                if page.is_selected() {
                                    // TODO clearly i should be able to simplify this right
                                    window.window_title.set_title(&page.title());
                                    buffer.set_status(&window.workspace_uri.borrow());
                                }
                            });
                        }
                    }
                ),
            );
        } else {
            buffer.try_save(&window.obj());
        }
    }
}
