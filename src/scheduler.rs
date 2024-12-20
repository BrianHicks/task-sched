use chrono::{DateTime, Datelike, Duration, Local, TimeZone};

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
                what: EventData::Offline,
            });

            let next_date = date + Duration::days(1);

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
                what: EventData::Offline,
            });

            date = next_date
        }

        new
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
}

#[derive(Debug)]
pub struct Event {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub what: EventData,
}

#[derive(Debug)]
pub enum EventData {
    Offline,
}
