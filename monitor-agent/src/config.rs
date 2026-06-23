use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceConfig {
    pub name:         String,
    pub display_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub poll_interval_secs: u64,
    pub history_hours:      u64,
    pub api_key:            String,
    pub services:           Vec<ServiceConfig>,
}

pub fn load(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}
