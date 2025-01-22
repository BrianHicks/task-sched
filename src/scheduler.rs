use crate::config::Config;
use crate::task::Task;
use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Weekday};
use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Write},
    ops::Div,
};

#[derive(Debug)]
pub struct Scheduler {
    // bounds
    start: DateTime<Local>,
    end: DateTime<Local>,

    // tasks
    tw_config: Config,
    tasks: HashMap<String, TimedTask>,
    outstanding_tasks: HashSet<String>,

    // result
    pub commitments: Vec<Event>,
}

const BREAK_TIME: Duration = Duration::minutes(5);

impl Scheduler {
    #[tracing::instrument(
        "Scheduler::new",
        skip(start, end, work_days, work_start, work_end, tw_config)
    )]
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
            outstanding_tasks: HashSet::new(),

            // result
            commitments: Vec::new(),
        };

        let (start_hour, start_minute) = work_start;
        let (end_hour, end_minute) = work_end;

        let mut date = new.start;
        while date <= new.end {
            let next_date = date + Duration::days(1);
            tracing::trace!(?date, "considering blocked times for date");

            if work_days.contains(&date.weekday()) {
                tracing::trace!(?date, "was a weekday");

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
                tracing::trace!(?date, "was a weekend");

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

        tracing::trace!(?new.commitments, "determined initial blocker commitments");

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
        self.outstanding_tasks.insert(task.uuid.clone());
        self.tasks.insert(
            task.uuid.clone(),
            TimedTask {
                remaining_time: task.estimate.unwrap_or(Duration::minutes(10)),
                task,
            },
        );
    }

    #[tracing::instrument(skip(self))]
    pub fn schedule(&mut self) {
        // Before we begin, make sure we don't have overlapping blocked time.
        self.simplify();

        let mut commitments = std::mem::take(&mut self.commitments);
        let mut outstanding_tasks = std::mem::take(&mut self.outstanding_tasks);

        let mut index = 0;
        let mut now = self.start;

        tracing::trace!(?index, ?now, "initial values");

        'scheduler: loop {
            while let Some(task) = commitments.get(index) {
                if task.start > now {
                    break;
                }
                index += 1;
                now = now.max(task.end);
                tracing::trace!(?index, ?now, "moved forward");
            }

            if now >= self.end {
                break;
            }

            let next_commitment = commitments.get(index).map(|t| t.start).unwrap_or(self.end);

            tracing::trace!(?next_commitment, "next commitment");

            let mut time_available = next_commitment - now;
            tracing::trace!(start=?now, ?time_available, "scheduling for slot");

            while time_available > Duration::zero() {
                if time_available <= BREAK_TIME {
                    tracing::debug!(start=?now, ?time_available, "scheduling short break");

                    commitments.insert(
                        index,
                        Event {
                            start: now,
                            end: now + time_available,
                            what: EventData::Break,
                        },
                    );

                    index += 1;
                    now += time_available;
                    time_available = Duration::zero();
                    continue;
                }

                match self.best_task_at(now, &outstanding_tasks) {
                    None => {
                        tracing::trace!("no tasks left; finishing");
                        break 'scheduler;
                    }
                    Some(task) => {
                        // If we have dependencies, this is a meta-task and
                        // should just be broken down or complete on the spot
                        // instead of having time schedule for it.
                        let time_for_task = if task.is_meta() {
                            time_available.min(Duration::minutes(10))
                        } else {
                            task.remaining_time.min(time_available)
                        };

                        let event = Event {
                            start: now,
                            end: now + time_for_task,
                            what: EventData::Task {
                                uuid: task.uuid.clone(),
                                name: task.description.clone(),
                                is_meta: task.is_meta(),
                            },
                        };
                        tracing::debug!(?event.start, ?time_for_task, ?event.what, "scheduled task");
                        commitments.insert(index, event);

                        index += 1;
                        now += time_for_task;
                        time_available -= time_for_task;

                        task.checked_sub(if task.is_meta() {
                            task.remaining_time
                        } else {
                            time_for_task
                        });

                        if !task.available() {
                            outstanding_tasks.remove(&task.uuid);
                        }
                    }
                }

                tracing::trace!(?time_available, "remaining time available");
            }

            tracing::trace!(?index, ?now, "done scheduling slot");
        }

        self.commitments = commitments;
        self.outstanding_tasks = outstanding_tasks;
    }

    pub fn simplify(&mut self) {
        let size = self.commitments.len();

        let mut old = std::mem::replace(&mut self.commitments, Vec::with_capacity(size));

        let mut iter = old.drain(..);
        let mut current = iter.next();

        while let Some(mut event) = current.take() {
            let next = iter.next();

            match &next {
                Some(next_event) => {
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

                    current = next;
                }
                None => self.commitments.push(event),
            }
        }
    }

    fn best_task_at(
        &mut self,
        when: DateTime<Local>,
        outstanding_tasks: &HashSet<String>,
    ) -> Option<&mut TimedTask> {
        self.tasks
            .values_mut()
            .filter(|task| task.available())
            .filter(|task| task.available_at(when.to_utc()))
            .filter(|task| {
                outstanding_tasks
                    .intersection(&task.depends)
                    .next()
                    .is_none()
            })
            .map(|task| {
                let urgency = task.urgency_at(when.to_utc(), &self.tw_config);

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

    fn is_meta(&self) -> bool {
        !self.depends.is_empty()
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

impl Event {
    pub fn duration(&self) -> Duration {
        self.end - self.start
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.what {
            EventData::Break => {
                f.write_str("---------- ")?;
                self.start.format("%-I:%M %P").fmt(f)?;
                f.write_str(" (")?;
                f.write_str(&human_time(self.duration()))?;
                f.write_str(" break) ----------\n")
            }

            EventData::Blocked => {
                f.write_str("\n========== ")?;
                self.start.format("%-I:%M %P").fmt(f)?;
                f.write_str(" (")?;
                f.write_str(&human_time(self.duration()))?;
                f.write_str(" block) ==========\n")
            }

            EventData::Task { name, is_meta, .. } => {
                self.start.format("%b %-d, %_I:%M %P").fmt(f)?;
                f.write_str(" (")?;

                let duration = human_time(self.duration());
                let mut pad = 3_usize.saturating_sub(duration.len());

                f.write_str(&duration)?;
                f.write_char(')')?;
                while pad > 0 {
                    f.write_char(' ')?;
                    pad -= 1;
                }

                f.write_str(" - ")?;

                if *is_meta {
                    f.write_str("META - ")?;
                }

                f.write_str(name)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EventData {
    Blocked,
    Break,
    Task {
        uuid: String,
        name: String,
        is_meta: bool,
    },
}

fn human_time(duration: Duration) -> String {
    let mut minutes = duration.num_minutes() as f64;

    if minutes < 60.0 {
        format!("{minutes}m")
    } else {
        let hours = minutes.div(60.0).floor();
        minutes -= hours * 60.0;

        // we could look for time longer than hours, but practically speaking we'll
        // barely ever get into hours territory so I'm not too concerned about it!
        if minutes > 0.0 {
            format!("{hours}h{minutes}m")
        } else {
            format!("{hours}h")
        }
    }
}
