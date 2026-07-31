use gtk::gio::{FileInfo, ThemedIcon};

pub fn from_file_info(file_info: &FileInfo) -> ThemedIcon {
    from_mime_type(file_info.content_type().unwrap().to_string())
}

pub fn from_mime_type(mime_type: String) -> ThemedIcon {
    // audio, video, images
    if mime_type.contains("audio") {
        return ThemedIcon::new("audio-x-generic-symbolic");
    }
    if mime_type.contains("video") {
        return ThemedIcon::new("video-x-generic-symbolic");
    }
    if mime_type.contains("image") {
        return ThemedIcon::new("image-x-generic-symbolic");
    }
    // text types (works best with custom icons; see gnome builder symbolic icons)
    if mime_type.contains("text") {
        let last = mime_type.split("/").last().unwrap();
        return ThemedIcon::from_names(&[
            &format!("text-{}-symbolic", last),
            &format!("text-{}", last),
            "text-x-generic-symbolic",
        ]);
    }
    // default
    ThemedIcon::new("text-x-generic-symbolic")
}
