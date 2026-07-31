#[derive(Debug, Clone)]
#[derive(Default)]
pub enum Status {
    #[default]
    Normal,
    Progress(f32),
    Done,
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub enum Priority {
    Low,
    #[default]
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


