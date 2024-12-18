mod task;
mod taskwarrior;
use std::process::ExitCode;

use clap::Parser;
use color_eyre::eyre::Result;
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
pub struct Cli {}

impl Cli {
    async fn run(&self) -> Result<()> {
        let tw = Taskwarrior::new("task".to_owned());

        println!(
            "{:#?}",
            tw.export()
                .with_urgency_coefficient("due", 0.0)
                .with_urgency_coefficient("age", 0.0)
                .with_filter("status:pending")
                .call()
                .await?
        );

        Ok(())
    }
}
