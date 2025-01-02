use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Weekday};
use color_eyre::eyre::{bail, Context, ContextCompat, OptionExt, Result};
use ical::parser::{ical::component::IcalEvent, Component};

#[derive(Debug)]
pub struct Scheduler {
    start: DateTime<Local>,
    end: DateTime<Local>,
    pub commitments: Vec<Event>,
}

impl Scheduler {
    pub fn new(
        start: DateTime<Local>,
        end: DateTime<Local>,
        work_days: Vec<Weekday>,
        work_start: (u8, u8),
        work_end: (u8, u8),
    ) -> Self {
        let mut new = Self {
            start,
            end,
            commitments: Vec::new(),
        };

        let (start_hour, start_minute) = work_start;
        let (end_hour, end_minute) = work_end;

        let mut date = new.start.clone();
        while date <= new.end {
            let next_date = date + Duration::days(1);

            if work_days.contains(&date.weekday()) {
                new.commitments.push(Event {
                    start: Local
                        .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
                        .unwrap(),
                    end: Local
                        .with_ymd_and_hms(
                            date.year(),
                            date.month(),
                            date.day(),
                            start_hour.into(),
                            start_minute.into(),
                            0,
                        )
                        .unwrap(),
                    what: EventData::Blocked,
                });

                new.commitments.push(Event {
                    start: Local
                        .with_ymd_and_hms(
                            date.year(),
                            date.month(),
                            date.day(),
                            end_hour.into(),
                            end_minute.into(),
                            0,
                        )
                        .unwrap(),
                    end: Local
                        .with_ymd_and_hms(
                            next_date.year(),
                            next_date.month(),
                            next_date.day(),
                            0,
                            0,
                            0,
                        )
                        .unwrap(),
                    what: EventData::Blocked,
                });
            } else {
                new.commitments.push(Event {
                    start: Local
                        .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
                        .unwrap(),
                    end: Local
                        .with_ymd_and_hms(
                            next_date.year(),
                            next_date.month(),
                            next_date.day(),
                            0,
                            0,
                            0,
                        )
                        .unwrap(),
                    what: EventData::Blocked,
                });
            }

            date = next_date
        }

        new
    }

    pub fn block(&mut self, start: DateTime<Local>, end: DateTime<Local>) {
        if end <= self.start || start >= self.end {
            return;
        }

        let new_event = Event {
            start,
            end,
            what: EventData::Blocked,
        };

        for (i, event) in self.commitments.iter().enumerate() {
            if event.start > new_event.start {
                self.commitments.insert(i, new_event);
                return;
            }
        }

        self.commitments.push(new_event);
    }

    pub fn schedule(&self) {
        /*

        in a loop while we have free space in the schedule:

        1. look at the earliest bit of free time we have
        2. constrain the free time to 30 minutes
        3. if we got 30 minutes exactly, schedule a five minute break at the end and shorten the duration by five minutes
        4. select the most urgent ready task (unblocked + not waiting) at the start time
        5. if that task will take longer than the available time, deduct the time from its estimate and schedule it then
        6. if that task is shorter than the available time, schedule it as a whole, increment the start time, add it to the resolved task list for blocker calculation, and repeat from 4

        We're not too concerned if we schedule things past their due dates. The urgency algorithm should take care of most of it for us.

        Soooo I need:

        1. a way to figure out what blocks of free time are available (or at least what the *next* block of free time is)
        2. a way to annotate tasks with provisionally resolved depdendencies and partial completion
        3. a way to store scheduling decisions

        */
    }

    pub fn simplify(&mut self) {
        let size = self.commitments.len();

        let mut old = std::mem::replace(&mut self.commitments, Vec::with_capacity(size));

        let mut iter = old.drain(..);
        let mut current = iter.next();

        while let Some(mut event) = current.take() {
            let next = iter.next();

            if let Some(next_event) = &next {
                if event.what == next_event.what
                    && event.end >= next_event.start
                    && event.start <= next_event.end
                {
                    event.start = event.start.min(next_event.start);
                    event.end = event.end.max(next_event.end);

                    current = Some(event);
                    continue;
                } else {
                    self.commitments.push(event)
                }
            }

            current = next;
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub what: EventData,
}

#[derive(Debug, PartialEq)]
pub enum EventData {
    Blocked,
    Calendar(String),
}

impl TryFrom<IcalEvent> for Event {
    type Error = color_eyre::eyre::Error;

    fn try_from(event: IcalEvent) -> Result<Self, Self::Error> {
        let start_raw = event
            .get_property("DTSTART")
            .and_then(|prop| prop.value.clone())
            .wrap_err("could not get event start")?;

        let start = from_ical_date(&start_raw)
            .wrap_err_with(|| format!("could not parse event start ({start_raw})"))?
            .into();

        let mut end_raw = event
            .get_property("DTEND")
            .and_then(|prop| prop.value.clone())
            .wrap_err("could not get event end")?
            .replace("Z", "");
        end_raw.push_str("+00:00");

        let end = from_ical_date(&end_raw)
            .wrap_err_with(|| format!("could not parse event end ({end_raw})"))?
            .into();

        let what = EventData::Calendar(
            event
                .get_property("SUMMARY")
                .and_then(|prop| prop.value.clone())
                .unwrap_or("Untitled Event".to_string()),
        );

        Ok(Self { start, end, what })
    }
}

const ICAL_DATE_FORMAT: &str = "%Y%m%dT%H%M%S%:z";

const ICAL_DATE_FORMAT_NO_TZ: &str = "%Y%m%dT%H%M%S";

const ICAL_DATE_FORMAT_DATE_ONLY_NO_TZ: &str = "%Y%m%d";

fn from_ical_date(s: &str) -> Result<DateTime<Local>> {
    if let Ok(dt) = DateTime::parse_from_str(&s.replace("Z", "+00:00"), ICAL_DATE_FORMAT) {
        Ok(dt.into())
    } else if let Ok(dt) = NaiveDateTime::parse_from_str(&s, ICAL_DATE_FORMAT_NO_TZ) {
        Ok(dt.and_utc().into())
    } else if let Ok(dt) = DateTime::parse_from_str(&s.replace("+", "T000000+"), ICAL_DATE_FORMAT) {
        Ok(dt.into())
    } else if let Ok(d) = NaiveDate::parse_from_str(&s, ICAL_DATE_FORMAT_DATE_ONLY_NO_TZ) {
        d.and_hms_opt(0, 0, 0)
            .ok_or_eyre("could not set {s} to midnight")
            .map(|dt| dt.and_utc().into())
    } else {
        bail!("could not parse {s} to any known date format");
    }
}
