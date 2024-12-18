use crate::task::Task;
use color_eyre::eyre::{Context, Result};
use std::collections::HashMap;
use tokio::process::Command;

pub struct Taskwarrior {
    binary: String,
}

impl Taskwarrior {
    pub fn new(binary: String) -> Self {
        Self { binary }
    }

    pub fn export(&self) -> ExportBuilder {
        ExportBuilder {
            binary: self.binary.clone(),
            urgency_coefficients: HashMap::new(),
            filters: Vec::new(),
        }
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

    pub async fn call(self) -> Result<Vec<Task>> {
        let mut command = Command::new(self.binary);

        for (key, coefficient) in self.urgency_coefficients {
            command.arg(format!("rc.urgency.{}.coefficient={}", key, coefficient));
        }

        command.args(self.filters);

        command.arg("export");

        let output = command
            .output()
            .await
            .wrap_err("could not retrieve tasks")?;

        println!("{:?}", String::from_utf8_lossy(&output.stdout));

        serde_json::from_slice(&output.stdout).wrap_err("could not deserialize tasks")
    }
}
