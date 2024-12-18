use crate::config::Config;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de::{self, Visitor};
use std::fmt;

#[derive(Debug, serde::Deserialize)]
pub struct Task {
    id: usize,
    uuid: String,

    description: String,
    status: Status,
    #[serde(default)]
    tags: Vec<String>,

    urgency: f64,

    #[serde(deserialize_with = "tw_datetime")]
    entry: DateTime<Utc>,
    #[serde(deserialize_with = "tw_datetime")]
    modified: DateTime<Utc>,
    #[serde(default, deserialize_with = "tw_datetime_opt")]
    wait: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "tw_datetime_opt")]
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

struct TWDateTimeVisitor;

impl<'de> Visitor<'de> for TWDateTimeVisitor {
    type Value = DateTime<Utc>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in the format YYYYMMDDTHHMMSSZ")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let naive =
            NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ").map_err(de::Error::custom)?;

        Ok(naive.and_utc())
    }
}

fn tw_datetime<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    d.deserialize_str(TWDateTimeVisitor)
}

struct OptionalTWDateTimeVisitor;

impl<'de> Visitor<'de> for OptionalTWDateTimeVisitor {
    type Value = Option<DateTime<Utc>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in the format YYYYMMDDTHHMMSSZ or null")
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Delegate to the existing datetime deserialization logic
        tw_datetime(deserializer).map(Some)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

fn tw_datetime_opt<'de, D>(d: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    d.deserialize_option(OptionalTWDateTimeVisitor)
}
