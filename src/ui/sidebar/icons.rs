use gtk::gio::FileInfo;

pub fn from_file_info(file_info: &FileInfo) -> String {
    from_mime_type(file_info.content_type().unwrap().to_string())
}

pub fn from_mime_type(mime_type: String) -> String {
    // audio, video, images
    if mime_type.contains("audio") {
        return "audio-x-generic-symbolic".into();
    }
    if mime_type.contains("video") {
        return "video-x-generic-symbolic".into();
    }
    if mime_type.contains("image") {
        return "image-x-generic-symbolic".into();
    }
    // text types (works best with custom icons; see gnome builder symbolic icons)
    if mime_type.contains("text") {
        let icon = format!("text-{}-symbolic", mime_type.split("/").last().unwrap());
        if gtk::IconTheme::default().has_icon(&icon) {
            return icon;
        }
    }
    // default
    "text-x-generic-symbolic".into()
}
