#[derive(Debug)]
pub enum Status {
    Normal,
    Progress(f32),
    Done,
}

#[derive(Debug)]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Default, Debug)]
pub struct Notification {
    pub status: Status,
    pub priority: Priority,
    pub summary: String,
    pub details: String,
    pub id: String,
}

impl Default for Status {
    fn default() -> Self {
        Status::Normal
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}
