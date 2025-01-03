use crate::config::Config;
use crate::task::Task;
use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Weekday};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Scheduler {
    // bounds
    start: DateTime<Local>,
    end: DateTime<Local>,

    // tasks
    tw_config: Config,
    tasks: HashMap<String, TimedTask>,

    // result
    pub commitments: Vec<Event>,
}

const SESSION_TIME: Duration = Duration::minutes(30);

const BREAK_TIME: Duration = Duration::minutes(5);

impl Scheduler {
    pub fn new(
        start: DateTime<Local>,
        end: DateTime<Local>,
        work_days: Vec<Weekday>,
        work_start: (u8, u8),
        work_end: (u8, u8),
        tw_config: Config,
    ) -> Self {
        let mut new = Self {
            // bounds
            start,
            end,

            // tasks
            tw_config,
            tasks: HashMap::new(),

            // result
            commitments: Vec::new(),
        };

        let (start_hour, start_minute) = work_start;
        let (end_hour, end_minute) = work_end;

        let mut date = new.start;
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

    pub fn add_task(&mut self, task: Task) {
        self.tasks.insert(
            task.uuid.clone(),
            TimedTask {
                remaining_time: task.estimate.unwrap_or(Duration::minutes(10)),
                task,
            },
        );
    }

    pub fn schedule(mut self, start: DateTime<Local>) -> Vec<Event> {
        // Before we begin, make sure we don't have overlapping blocked time.
        self.simplify();

        let mut commitments = std::mem::replace(&mut self.commitments, Vec::new());

        let mut index = 1;
        let mut start = start.max(self.start);

        while let Some(task) = commitments.get(index) {
            if task.start >= start {
                break;
            }
            index += 1;
        }

        'scheduler: loop {
            if index == commitments.len() {
                break;
            }

            start = commitments
                .get(index - 1)
                .map(|t| t.end)
                .unwrap_or(self.start)
                .max(start);

            println!("starting to schedule at {start}");

            let next_commitment = self
                .commitments
                .get(index)
                .map(|t| t.start)
                .unwrap_or(self.end);

            println!("next commitment is at {next_commitment}");

            let mut time_available = (next_commitment - start).min(SESSION_TIME);
            let can_schedule_break = time_available == SESSION_TIME;
            if can_schedule_break {
                time_available -= BREAK_TIME;
            }

            while time_available > Duration::zero() {
                println!("I have {time_available} available");

                match self.best_task_at(start, time_available) {
                    None => break 'scheduler,
                    Some(task) => {
                        let time_for_task = task.remaining_time.min(time_available);

                        commitments.insert(
                            index,
                            Event {
                                start,
                                end: start + time_for_task,
                                what: EventData::Task(task.uuid.clone(), task.description.clone()),
                            },
                        );

                        index += 1;
                        start += time_for_task;
                        time_available -= time_for_task;
                        task.checked_sub(time_for_task);
                    }
                }
            }

            if can_schedule_break {
                commitments.insert(
                    index,
                    Event {
                        start,
                        end: start + BREAK_TIME,
                        what: EventData::Break,
                    },
                );
                index += 1;
            }

            println!("done with commitments, index is at {index}, start is at {start}");

            // TODO: increment index etc etc
            if index > 50 {
                break;
            }
            // break;
        }

        commitments

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

    fn best_task_at(
        &mut self,
        when: DateTime<Local>,
        time_available: Duration,
    ) -> Option<&mut TimedTask> {
        self.tasks
            .values_mut()
            .filter(|task| task.available())
            .filter(|task| task.task.available_at(when.to_utc()))
            .map(|task| {
                let mut urgency = task.urgency_at(when.to_utc(), &self.tw_config);
                urgency -= ((task.remaining_time.num_minutes() as f64
                    - time_available.num_minutes() as f64)
                    / time_available.num_minutes() as f64)
                    .abs()
                    .powf(2.0)
                    .min(3.0);

                (task, urgency)
            })
            .max_by(|(_, urg_a), (_, urg_b)| urg_a.total_cmp(urg_b))
            .map(|(task, _)| task)
    }
}

#[derive(Debug)]
struct TimedTask {
    task: Task,
    remaining_time: Duration,
}

impl std::ops::Deref for TimedTask {
    type Target = Task;

    fn deref(&self) -> &Self::Target {
        &self.task
    }
}

impl TimedTask {
    fn available(&self) -> bool {
        self.remaining_time > Duration::zero()
    }

    fn checked_sub(&mut self, how_much: Duration) {
        self.remaining_time = Duration::zero().max(self.remaining_time - how_much);
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
    Break,
    Task(String, String),
}
