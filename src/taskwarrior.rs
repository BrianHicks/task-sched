use crate::{config::Config, task::Task};
use color_eyre::eyre::{Context, Result};
use std::collections::HashMap;
use tokio::process::Command;

#[derive(Debug)]
pub struct Taskwarrior {
    binary: String,
}

impl Taskwarrior {
    pub fn new(binary: String) -> Self {
        Self { binary }
    }

    #[tracing::instrument]
    pub fn export(&self) -> ExportBuilder {
        ExportBuilder {
            binary: self.binary.clone(),
            urgency_coefficients: HashMap::new(),
            filters: Vec::new(),
        }
    }

    #[tracing::instrument]
    pub async fn config(&self) -> Result<Config> {
        let mut command = Command::new(&self.binary);
        command.arg("_show");

        tracing::trace!(?command, "getting config from taskwarrior");

        let output = command
            .output()
            .await
            .wrap_err("could not call Taskwarrior")?;

        let config_text =
            String::from_utf8(output.stdout).wrap_err("config contained invalid UTF-8")?;

        Config::parse(&config_text).wrap_err("could not parse config")
    }
}

pub struct ExportBuilder {
    binary: String,
    urgency_coefficients: HashMap<String, f64>,
    filters: Vec<String>,
}

impl ExportBuilder {
    pub fn with_urgency_coefficient(mut self, key: &str, value: f64) -> Self {
        self.urgency_coefficients.insert(key.to_owned(), value);

        self
    }

    pub fn with_filter(mut self, filter: &str) -> Self {
        self.filters.push(filter.to_owned());

        self
    }

    #[tracing::instrument("export", skip(self))]
    pub async fn call(self) -> Result<Vec<Task>> {
        let mut command = Command::new(self.binary);

        for (key, coefficient) in self.urgency_coefficients {
            command.arg(format!("rc.urgency.{}.coefficient={}", key, coefficient));
        }

        command.args(self.filters);

        command.arg("export");

        tracing::trace!(?command, "calling taskwarrior for export");

        let output = command
            .output()
            .await
            .wrap_err("could not retrieve tasks")?;

        serde_json::from_slice(&output.stdout).wrap_err("could not deserialize tasks")
    }
}
