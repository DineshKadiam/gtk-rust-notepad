use adw::{TabPage, prelude::*, subclass::prelude::*};
use async_lsp::lsp_types::{
    DocumentChangeOperation, DocumentChanges, TextDocumentEdit, WorkspaceEdit,
};
use gtk::{
    FileLauncher, Label,
    gio::{self, Cancellable, SimpleAction},
    glib::{self, Priority, Variant, clone},
};
use sourceview5::prelude::*;
use vte4::prelude::*;

use crate::{
    lsp,
    ui::{
        buffer::Buffer, dialog::preferences::PreferencesDialog, dialog::recents::RecentsDialog,
        dialog::rename_file, terminal::TerminalView, window::EditorWindow,
    },
    utils,
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
    let path = {
        let p = param.unwrap().to_string();
        p[1..(p.len() - 1)].to_string()
    };
    window.imp().new_tab(Some(&path));
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
    page.set_icon(Some(&gio::ThemedIcon::from_names(&[
        "utilities-terminal-symbolic",
    ])));
    if page.is_selected() {
        window.imp().window_title.set_title(&page.title());
    }
    window.imp().stack.set_visible_child_name("tabs");
    // Connect to vte title changes
    let window = window.clone();
    term.imp().terminal.connect_child_exited(clone!(
        #[strong]
        page,
        #[strong(rename_to=tab_view)]
        window.imp().tab_view,
        move |_, code| {
            // If the child closed without any error code (e.g. didn't crash),
            // we can assume it was intentional; e.g. the user is done with
            // the terminal/view and doesn't need it anymore.
            if code == 0 {
                tab_view.close_page(&page);
            }
        }
    ));
    term.imp()
        .terminal
        .connect_termprop_changed(Some("xterm.title"), move |term, _| {
            let title = term
                .termprop_string("xterm.title")
                .0
                .unwrap_or("Terminal".into());
            page.set_title(&title);
            if page.is_selected() {
                window.imp().window_title.set_title(&page.title());
            }
        });
}

/// Reloads tab
pub fn tab_reload(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(page) = window.imp().tab_view.selected_page()
        && let Some(buffer) = page.child().downcast_ref::<Buffer>()
    {
        buffer.reload_file(false);
    }
}

/// Closes the current tab
pub fn tab_close(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(page) = window.imp().tab_view.selected_page() {
        // LSP file close
        if let Some(buffer) = page.child().downcast_ref::<Buffer>()
            && let Some(path) = buffer.file_path()
        {
            lsp::file::close(&path, &window.workspace_uri());
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
                    let path = file.path().unwrap().to_str().unwrap().to_string();
                    window.imp().new_tab(Some(&path));
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

/// Sets a folder (open_folder but not interactive)
pub fn set_folder(window: &EditorWindow, _: &SimpleAction, param: Option<&Variant>) {
    let path = {
        let p = param.unwrap().to_string();
        p[1..(p.len() - 1)].to_string()
    };
    window.update_folder(&path);
    // close tabs + open sidebar
    window.imp().split_view.set_show_sidebar(true);
    for page in window.imp().tab_view.pages().iter::<TabPage>() {
        window.imp().tab_view.close_page(&page.unwrap());
    }
}

/// Opens the preferences dialog
pub fn show_prefs(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    let prefs = PreferencesDialog::new();
    prefs.present(&window.clone().into());
}

/// Opens the recents dialog
pub fn show_recents(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    let prefs = RecentsDialog::new();
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

/// Open file tree URI with system handler
pub fn tree_launch(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(path) = window.imp().sidebar.get_selected_uri() {
        let window = window.clone();
        let file = gio::File::for_path(path);
        let launcher = FileLauncher::new(Some(&file));
        glib::spawn_future_local(async move { launcher.launch_future(Some(&window)).await });
    }
}

/// Open folder from file tree
pub fn tree_open_folder(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(path) = window.imp().sidebar.get_selected_uri() {
        window.update_folder(&path);
    }
}

/// Open file from file tree
pub fn tree_edit(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(path) = window.imp().sidebar.get_selected_uri() {
        window.imp().new_tab(Some(&path));
    }
}

/// Toggle a tree expander row to be expanded
pub fn tree_toggle_expand(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    window.imp().sidebar.tree_toggle_expand();
}

/// Create file from file tree
pub fn tree_new_file(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(path) = window.imp().sidebar.get_selected_uri() {
        let dialog = rename_file::new(rename_file::Kind::Create, path);
        let window: &gtk::Window = &window.clone().into();
        adw::prelude::AdwDialogExt::present(&dialog, Some(window));
    }
}

/// Rename file/folder in file tree
pub fn tree_rename(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(path) = window.imp().sidebar.get_selected_uri() {
        // Unparent popover (if exists)
        window.imp().sidebar.imp().folder_popover.unparent();
        window.imp().sidebar.imp().file_popover.unparent();
        // Show dialog
        let dialog = rename_file::new(rename_file::Kind::Rename, path);
        let window: &gtk::Window = &window.clone().into();
        adw::prelude::AdwDialogExt::present(&dialog, Some(window));
    }
}

/// Moves a selected file from file tree into the user's trash directory.
pub fn tree_delete_file(window: &EditorWindow, _: &SimpleAction, _: Option<&Variant>) {
    if let Some(path) = window.imp().sidebar.get_selected_uri() {
        // Unparent popover (if exists)
        window.imp().sidebar.imp().folder_popover.unparent();
        window.imp().sidebar.imp().file_popover.unparent();
        glib::spawn_future_local(async move {
            gio::File::for_path(path).trash_future(Priority::HIGH).await
        });
    }
}

/// Saves the current buffer
pub fn save_current_buffer(window: &EditorWindow, save_as: bool) {
    let window = window.imp();
    if let Some(page) = window.tab_view.selected_page()
        && let Some(buffer) = page.child().downcast_ref::<Buffer>()
    {
        if buffer.file_path().is_none() || save_as {
            let old_path = buffer.file_path();
            let workspace_uri = window.workspace_uri.borrow().clone();
            gtk::FileDialog::new().save(
                Some::<&gtk::Window>(&window.obj().clone().into()),
                Some(&Cancellable::new()),
                clone!(
                    #[strong]
                    buffer,
                    #[weak]
                    window,
                    move |result| {
                        if let Ok(file) = result {
                            let file_name =
                                file.path().unwrap().to_str().unwrap().to_string().clone();
                            // Set buffer path
                            buffer.set_file(&file_name);
                            // Syntax highlighting
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
                            // Save
                            buffer.save(&window.obj().clone());
                            if let Some(ref path) = old_path {
                                lsp::file::close(path, &workspace_uri);
                            }
                        }
                    }
                ),
            );
        } else {
            buffer.try_save(&window.obj());
        }
    }
}

/// Saves a buffer asynchronously. Used/designed for alert dialogs where LSP integration does not matter
pub async fn save_buffer_async(window: &EditorWindow, buffer: Buffer) {
    let window = window.imp();
    if buffer.file_path().is_some() {
        let (tx, rx) = async_channel::bounded(1);
        gtk::FileDialog::new().save(
            Some::<&gtk::Window>(&window.obj().clone().into()),
            Some(&Cancellable::new()),
            move |result| {
                if let Ok(file) = result {
                    tx.send_blocking(Some(
                        file.path().unwrap().to_str().unwrap().to_string().clone(),
                    ))
                    .unwrap();
                } else {
                    tx.send_blocking(None).unwrap();
                }
            },
        );
        if let Ok(Some(file_name)) = rx.recv().await {
            buffer.set_file(&file_name);
            buffer.write().await;
        }
    } else {
        buffer.write().await;
    }
}

// Handle LSP workspace edits
pub fn lsp_edit(window: &EditorWindow, _: &SimpleAction, json: Option<&Variant>) {
    let workspace_edit: WorkspaceEdit = {
        let json = json.unwrap().get::<String>().unwrap();
        serde_json::from_str(&json).unwrap()
    };
    match workspace_edit.document_changes {
        Some(DocumentChanges::Edits(edits)) => {
            for edit in edits {
                lsp_document_edit(window, edit);
            }
        }
        Some(DocumentChanges::Operations(operations)) => {
            for operation in operations {
                match operation {
                    DocumentChangeOperation::Op(_) => todo!(), // TODO: We need to have an internal file management API before support for this
                    DocumentChangeOperation::Edit(edit) => lsp_document_edit(window, edit),
                }
            }
        }
        _ => (),
    }
}

fn lsp_document_edit(window: &EditorWindow, edit: TextDocumentEdit) {
    let edit_path = edit.text_document.uri.path().to_string();
    for page in window.imp().tab_view.pages().iter::<TabPage>() {
        if let Ok(page) = page
            && let Ok(buffer) = page.child().downcast::<Buffer>()
            && buffer
                .file_path()
                .is_some_and(|ref path| &edit_path == path)
        {
            buffer.apply_document_edit(edit.clone());
            return;
        }
    }
    // If we reached this there wasn't a buffer open with the edit.
    // Open a new buffer and apply the edit
    let page = window.imp().new_tab(Some(&edit_path));
    if let Ok(buffer) = page.child().downcast::<Buffer>() {
        buffer.apply_document_edit(edit);
    }
}
