mod config;
mod task;
mod taskwarrior;

use chrono::Utc;
use clap::Parser;
use color_eyre::eyre::{Context, Result};
use config::Config;
use std::process::ExitCode;
use taskwarrior::Taskwarrior;

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
}

impl Cli {
    async fn run(&self) -> Result<()> {
        let tw = Taskwarrior::new(self.taskwarrior_binary.clone());

        let config = tw.config().await.wrap_err("could not get config")?;

        println!(
            "{:#?}",
            tw.export()
                .with_urgency_coefficient("due", 0.0)
                .with_urgency_coefficient("age", 0.0)
                .with_urgency_coefficient("blocked", 0.0)
                .with_urgency_coefficient("blocking", 0.0)
                .with_filter("status:pending")
                .with_filter("due.any:")
                .call()
                .await?
                .first()
                .map(|t| t.urgency_at(Utc::now(), &config))
        );

        Ok(())
    }
}
