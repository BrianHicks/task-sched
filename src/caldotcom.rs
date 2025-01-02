use color_eyre::eyre::{Result, WrapErr};
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
}

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    status: String,
    data: T,
}

#[derive(Debug, Deserialize)]
pub struct Calendars {
    connectedCalendars: Vec<CalendarConnection>,
}

#[derive(Debug, Deserialize)]
pub struct CalendarConnection {
    credentialId: usize,
    calendars: Vec<ConnectedCalendar>,
}

#[derive(Debug, Deserialize)]
pub struct ConnectedCalendar {
    externalId: String,
    isSelected: bool,
}
