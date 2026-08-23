// Buffer file I/O functions
use super::*;

impl Buffer {
    /// Asynchronously writes the buffer to `Buffer::file_path` and updates both the associated window and LSP.
    pub fn save(&self, window: &EditorWindow) {
        let workspace_uri = window.imp().workspace_uri.borrow().clone();
        glib::spawn_future_local(clone!(
            #[weak]
            window,
            #[weak(rename_to=buffer)]
            self,
            async move {
                let selected_page = window.imp().tab_view.selected_page();
                // Save, and then...
                buffer.write().await;
                // Tell LSP file was modified
                let path = buffer.file_path().unwrap();
                lsp::file::change(&path, &workspace_uri, None);
                lsp::file::save(&path, &workspace_uri);
                // Format and reload buffer
                if lsp::format::format(&path, &workspace_uri).await {
                    buffer.reload_file(true);
                }
                // Set window/tab title
                if let Some(page) = selected_page {
                    page.set_title(&buffer.tab_name());
                    if let Some(ref file) = *buffer.imp().file.borrow()
                        && let Ok(info) = file.location().unwrap().query_info(
                            "standard::*",
                            FileQueryInfoFlags::NONE,
                            Cancellable::NONE,
                        )
                    {
                        let icon = sidebar::icons::from_file_info(&info);
                        page.set_icon(Some(&icon));
                    }
                    if page.is_selected() {
                        // TODO clearly i should be able to simplify this right
                        window.imp().window_title.set_title(&page.title());
                        buffer.set_status(&window.imp().workspace_uri.borrow());
                    }
                }
            }
        ));
    }

    /// Force writes buffer to a file.
    pub async fn write(&self) {
        if let Some(ref file) = self.imp().file.borrow().clone() {
            let save = sourceview5::FileSaver::new(
                self.imp()
                    .source_view
                    .buffer()
                    .downcast_ref::<sourceview5::Buffer>()
                    .unwrap(),
                file,
            );
            save.set_flags(FileSaverFlags::IGNORE_MODIFICATION_TIME);
            save.save_future(glib::Priority::default()).0.await.unwrap();
        }
    }

    /// Tries to save the file -- if there is a conflict (the file was edited externally), show an alert
    pub fn try_save(&self, window: &EditorWindow) {
        if let Some(ref file) = *self.imp().file.borrow() {
            // Checks if file is externally modified
            file.check_file_on_disk();
            if file.is_externally_modified() {
                let dialog = AlertDialog::new(
                    Some("File modified..."),
                    Some(&format!(
                        "<tt>{}</tt> has been modified by an external program. Really save?",
                        self.clean_name(&window.imp().workspace_uri.borrow().clone())
                    )),
                );
                dialog.set_body_use_markup(true);
                let gtk_window: &gtk::Window = &window.clone().into();
                dialog.add_responses(&[
                    ("cancel", "_Cancel"),
                    ("save", "_Force save"),
                    ("reload", "_Force reload"),
                ]);
                dialog.set_response_appearance("save", ResponseAppearance::Destructive);
                dialog.set_response_appearance("reload", ResponseAppearance::Destructive);
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");
                dialog.connect_response(
                    Some("save"),
                    clone!(
                        #[weak(rename_to=buffer)]
                        self,
                        #[strong]
                        window,
                        move |_, _| {
                            buffer.save(&window);
                        }
                    ),
                );
                dialog.connect_response(
                    Some("reload"),
                    clone!(
                        #[weak(rename_to=buffer)]
                        self,
                        move |_, _| {
                            buffer.reload_file(true);
                        }
                    ),
                );
                adw::prelude::AdwDialogExt::present(&dialog, Some(gtk_window));
            } else {
                self.save(window);
            }
        }
    }

    /// Reloads current file
    pub fn reload_file(&self, reversible: bool) {
        if let Some(path) = self.file_path() {
            self.set_buffer_text(&fs::read_to_string(&path).unwrap(), reversible);
            self.set_file(&path);
        }
    }

    /// Loads a file from a path. The loading process is spawned in another thread.
    /// TODO make this an alias to load_file_async
    pub fn load_file(&self, path: &str) {
        let buffer = self.get_sourceview_buffer();
        let gio_file = gio::File::for_path(path);
        let file = sourceview5::File::builder().location(&gio_file).build();
        let info = gio_file
            .query_info("standard::*", FileQueryInfoFlags::NONE, Cancellable::NONE)
            .unwrap();
        let loader = sourceview5::FileLoader::new(&buffer, &file);
        loader.load_async(glib::Priority::default(), gio::Cancellable::NONE, |_| {});
        // Set file instance
        self.imp().file.replace(Some(file));
        // Set syntax highlighting
        if let Some(ref language) = sourceview5::LanguageManager::new()
            .guess_language(Some(&path), info.content_type().as_deref())
        {
            buffer.set_language(Some(language));
        }
    }

    /// Loads a file from a path, asynchronously
    pub async fn load_file_async(&self, path: &str) {
        let buffer = self.get_sourceview_buffer();
        let gio_file = gio::File::for_path(path);
        let file = sourceview5::File::builder().location(&gio_file).build();
        let info = gio_file
            .query_info("standard::*", FileQueryInfoFlags::NONE, Cancellable::NONE)
            .unwrap();
        let loader = sourceview5::FileLoader::new(&buffer, &file);
        // Set file instance
        self.imp().file.replace(Some(file));
        // Set syntax highlighting
        if let Some(ref language) = sourceview5::LanguageManager::new()
            .guess_language(Some(&path), info.content_type().as_deref())
        {
            buffer.set_language(Some(language));
        }
        // Load
        loader
            .load_future(glib::Priority::default())
            .0
            .await
            .unwrap();
    }
}
