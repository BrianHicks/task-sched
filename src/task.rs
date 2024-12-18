use crate::config::Config;
use chrono::{DateTime, Utc};

#[derive(Debug, serde::Deserialize)]
pub struct Task {
    // id: usize,
    // uuid: String,
    // description: String,
    // status: Status,
    // #[serde(default)]
    // tags: Vec<String>,
    urgency: f64,

    // #[serde(deserialize_with = "crate::dates::tw_datetime")]
    // entry: DateTime<Utc>,
    // #[serde(deserialize_with = "crate::dates::tw_datetime")]
    // modified: DateTime<Utc>,
    // #[serde(default, deserialize_with = "crate::dates::tw_datetime_opt")]
    // wait: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "crate::dates::tw_datetime_opt")]
    due: Option<DateTime<Utc>>,
    // long-tail fields: priority, project
}

impl Task {
    pub fn urgency_at(&self, when: DateTime<Utc>, coefficients: &Config) -> f64 {
        self.urgency + self.due_urgency_at(when, coefficients.urgency_due_coefficient)
    }

    fn due_urgency_at(&self, when: DateTime<Utc>, coefficient: f64) -> f64 {
        match self.due {
            Some(due) => {
                let days_overdue = (when - due).num_seconds() as f64 / 86_400.0;

                (if days_overdue >= 7.0 {
                    1.0
                } else if days_overdue >= -14.0 {
                    ((days_overdue + 14.0) * 0.8 / 21.0) + 0.2
                } else {
                    0.2
                } * coefficient)
            }
            None => 0.0,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pending,
    Completed,
    Deleted,
    Recurring,
}
