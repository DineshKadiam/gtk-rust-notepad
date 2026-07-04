#[derive(Debug, Clone)]
pub enum Status {
    Normal,
    Progress(f32),
    Done,
}

#[derive(Debug, Clone)]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Default, Debug, Clone)]
pub struct Notification {
    pub status: Status,
    pub priority: Priority,
    pub summary: String,
    pub details: String,
    pub id: String,
}

#[derive(Clone)]
pub enum Event {
    Push(Notification),
    Pop(String),
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
