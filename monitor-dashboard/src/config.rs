use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub ssh_host:      String,
    pub ssh_key:       String,
    pub api_key:       String,
    pub local_port:    u16,
    pub refresh_secs:  u64,
    pub history_hours: u64,
}

pub fn load(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}
