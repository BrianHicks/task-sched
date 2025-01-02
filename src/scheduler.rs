use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Weekday};

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

    pub fn schedule(&mut self) {
        // Before we begin, make sure we don't have overlapping blocked time.
        self.simplify();

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
