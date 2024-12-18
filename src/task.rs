use chrono::{DateTime, Utc};

#[derive(serde::Deserialize)]
pub struct Task {
    id: usize,
    uuid: String,

    description: String,
    status: Status,

    urgency: f64,

    entry: DateTime<Utc>,
    modified: Option<DateTime<Utc>>,
    wait: Option<DateTime<Utc>>,
    due: Option<DateTime<Utc>>,
    // long-tail fields: priority, project
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pending,
    Completed,
    Deleted,
    Recurring,
}
