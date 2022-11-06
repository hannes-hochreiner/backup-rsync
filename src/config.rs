use crate::{custom_duration::CustomDuration, ssh_credentials::SshCredentials};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs::File, path::Path};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub source: String,
    pub destination: String,
    pub exclude_file: String,
    pub log_file: String,
    pub ssh_credentials: SshCredentials,
    pub snapshot: String,
    pub snapshot_suffix: String,
    pub policy: Vec<CustomDuration>,
}

impl Config {
    pub fn read_from_file(filepath: &Path) -> Result<Self> {
        let file = File::open(filepath).context(format!(
            "could not open configuration file \"{}\"",
            filepath.to_string_lossy()
        ))?;

        Ok(serde_json::from_reader(file)?)
    }
}
