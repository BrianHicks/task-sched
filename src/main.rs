mod caldotcom;
// mod config;
// mod dates;
mod scheduler;
// mod task;
// mod taskwarrior;

use chrono::{Duration, Local, Weekday};
use clap::Parser;
use color_eyre::eyre::{Context, Result};
use scheduler::Scheduler;
use std::{path::PathBuf, process::ExitCode};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(err) = cli.run().await {
        eprintln!("{err:?}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[derive(clap::Parser)]
pub struct Cli {
    /// The location of the `task` binary to use for modifications.
    #[clap(long, default_value = "task")]
    taskwarrior_binary: String,

    /// The amount of days in the future to schedule.
    #[clap(long, default_value = "7")]
    days_out: u32,

    #[clap(long)]
    cal_token: String,
}

impl Cli {
    async fn run(&self) -> Result<()> {
        let start = Local::now();
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

        let mut scheduler = Scheduler::new(start, end, work_days, work_start, work_end);
        scheduler.schedule();
        scheduler.simplify();

        // add calendar events
        let client = caldotcom::CalDotCom::new(self.cal_token.clone());

        let calendars = client.calendars().await?;

        println!("{calendars:#?}");

        // for commitment in scheduler.commitments {
        //     println!(
        //         "{} - {}: {:?}",
        //         commitment.start.to_rfc2822(),
        //         commitment.end.to_rfc2822(),
        //         commitment.what
        //     )
        // }

        ///////////////////////

        // for cal_res in ical::IcalParser::new(ical_src) {
        //     let cal = cal_res?;

        //     for event in cal.events {
        //         println!(
        //             "{:} -- {:} to {:} {:?}",
        //             event
        //                 .get_property("SUMMARY")
        //                 .and_then(|p| p.value.clone())
        //                 .unwrap_or("NO SUMMARY".to_string()),
        //             event
        //                 .get_property("DTSTART")
        //                 .and_then(|p| p.value.clone())
        //                 .unwrap_or("NO DSTART".to_string()),
        //             event
        //                 .get_property("DTEND")
        //                 .and_then(|p| p.value.clone())
        //                 .unwrap_or("NO DTEND".to_string()),
        //             event
        //                 .properties
        //                 .iter()
        //                 .filter(|prop| prop.name == "ATTENDEE")
        //                 .filter(|prop| prop.value
        //                     == Some("mailto:brian.hicks@paynearme.com".to_string()))
        //                 .map(|prop| prop
        //                     .params
        //                     .clone()
        //                     .unwrap_or_else(|| Vec::new())
        //                     .drain(..)
        //                     .filter(|(k, _)| k == "PARTSTAT")
        //                     .map(|(_, v)| v)
        //                     .flatten()
        //                     .collect())
        //                 .collect::<Vec<Vec<String>>>(),
        //         );
        //     }
        // }

        ///////////////////////
        // let tw = Taskwarrior::new(self.taskwarrior_binary.clone());

        // let config = tw.config().await.wrap_err("could not get config")?;

        // println!(
        //     "{:#?}",
        //     tw.export()
        //         .with_urgency_coefficient("due", 0.0)
        //         .with_urgency_coefficient("age", 0.0)
        //         .with_urgency_coefficient("blocked", 0.0)
        //         .with_urgency_coefficient("blocking", 0.0)
        //         .with_filter("status:pending")
        //         .with_filter("due.any:")
        //         .call()
        //         .await?
        //         .iter()
        //         .map(|t| (t.description.clone(), t.urgency_at(Utc::now(), &config)))
        //         .collect::<Vec<(String, f64)>>()
        // );

        Ok(())
    }
}
