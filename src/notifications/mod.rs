pub mod data;

pub fn push(notif: data::Notification) {
    println!(
        "[NOTIF] [{:?}]{} {} (id: {})",
        notif.priority,
        if let data::Status::Progress(s) = notif.status {
            format!(" [{}%]", s * 100.)
        } else {
            String::new()
        },
        notif.summary,
        notif.id
    );
}
