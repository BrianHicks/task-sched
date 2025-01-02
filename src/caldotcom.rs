use chrono::{DateTime, Local};
use color_eyre::eyre::{Result, WrapErr};
use reqwest::Url;
use serde::Deserialize;

pub struct CalDotCom {
    token: String,
}

impl CalDotCom {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    pub async fn calendars(&self) -> Result<Response<Calendars>> {
        reqwest::Client::new()
            .get("https://api.cal.com/v2/calendars")
            .header("Authorization", format!("Bearer {}", &self.token))
            .send()
            .await
            .wrap_err("could not fetch calendars")?
            .json()
            .await
            .wrap_err("could not load calendars from JSON")
    }

    pub async fn busy_times(
        &self,
        calendars: Calendars,
        start: DateTime<Local>,
        end: DateTime<Local>,
    ) -> Result<Response<Vec<BlockedTime>>> {
        let mut params: Vec<(String, String)> = Vec::with_capacity(7);
        params.push(("loggedInUsersTz".into(), "America/Chicago".into()));
        params.push((
            "dateFrom".into(),
            start.date_naive().format("%Y-%m-%d").to_string(),
        ));
        params.push((
            "dateTo".into(),
            end.date_naive().format("%Y-%m-%d").to_string(),
        ));

        let mut calendars_to_fetch = Vec::with_capacity(2);
        for connection in calendars.connected_calendars {
            for calendar in connection.calendars {
                if calendar.is_selected {
                    calendars_to_fetch
                        .push((connection.credential_id.to_string(), calendar.external_id));
                }
            }
        }
        for (i, (cred, external_id)) in calendars_to_fetch.drain(..).enumerate() {
            params.push((format!("calendarsToLoad[{i}][credentialId]"), cred));
            params.push((format!("calendarsToLoad[{i}][externalId]"), external_id))
        }

        let url = Url::parse_with_params("https://api.cal.com/v2/calendars/busy-times", params)
            .wrap_err("could not construct busy-times URL")?;

        reqwest::Client::new()
            .get(url)
            .header("Authorization", format!("Bearer {}", &self.token))
            .send()
            .await
            .wrap_err("could not fetch busy-times")?
            .json()
            .await
            .wrap_err("could not load busy-times from JSON")
    }
}

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct Calendars {
    #[serde(alias = "connectedCalendars")]
    pub connected_calendars: Vec<CalendarConnection>,
}

#[derive(Debug, Deserialize)]
pub struct CalendarConnection {
    #[serde(alias = "credentialId")]
    pub credential_id: usize,
    pub calendars: Vec<ConnectedCalendar>,
}

#[derive(Debug, Deserialize)]
pub struct ConnectedCalendar {
    #[serde(alias = "externalId")]
    pub external_id: String,
    #[serde(alias = "isSelected")]
    pub is_selected: bool,
}

#[derive(Debug, Deserialize)]
pub struct BlockedTime {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}
