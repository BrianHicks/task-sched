mod caldotcom;
mod config;
mod dates;
mod scheduler;
mod task;
mod taskwarrior;

use chrono::{Duration, Local, Timelike, Weekday};
use clap::Parser;
use color_eyre::eyre::{Context, Result};
use scheduler::Scheduler;
use std::process::ExitCode;
use task::Status;
use taskwarrior::Taskwarrior;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let filter = EnvFilter::builder()
        .with_env_var("TASK_SCHED_LOG")
        .with_default_directive(cli.log_level.into())
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .init();

    if let Err(err) = cli.run().await {
        eprintln!("{err:?}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[derive(clap::Parser)]
#[clap(version, about)]
pub struct Cli {
    /// The location of the `task` binary to use for modifications.
    #[clap(long, default_value = "task")]
    taskwarrior_binary: String,

    /// The amount of days in the future to schedule.
    #[clap(long, default_value = "3")]
    days_out: u32,

    #[clap(long, env)]
    cal_token: String,

    #[clap(long, default_value = "info")]
    log_level: LevelFilter,
}

impl Cli {
    async fn run(&self) -> Result<()> {
        let start = Local::now()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();
        let end = start + Duration::days(self.days_out.into());

        // TODO: figure out how to parse these
        let work_days = vec![
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
        ];
        let work_start = (9, 0);
        let work_end = (17, 30);

        let tw = Taskwarrior::new(self.taskwarrior_binary.clone());

        let tw_config = tw.config().await.wrap_err("could not get config")?;

        let mut scheduler = Scheduler::new(start, end, work_days, work_start, work_end, tw_config);

        // add calendar events
        let client = caldotcom::CalDotCom::new(self.cal_token.clone());

        let calendars = client.calendars().await?;
        for busy_time in client.busy_times(calendars.data, start, end).await?.data {
            scheduler.block(busy_time.start, busy_time.end);
        }

        tw.export()
            .with_urgency_coefficient("due", 0.0)
            .with_urgency_coefficient("age", 0.0)
            .with_urgency_coefficient("blocked", 0.0)
            .with_urgency_coefficient("blocking", 0.0)
            .call()
            .await?
            .drain(..)
            .filter(|t| t.status == Status::Pending)
            .for_each(|t| scheduler.add_task(t));

        scheduler.schedule();

        for commitment in scheduler.commitments {
            if commitment.duration() <= Duration::minutes(120) {
                println!("{}", commitment)
            }
        }

        Ok(())
    }
}
