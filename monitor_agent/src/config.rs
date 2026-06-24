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
    #[serde(default)]
    pub services:           Vec<ServiceConfig>,
}

pub fn load(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example_config() {
        let toml_str = r#"
poll_interval_secs = 30
history_hours = 24
api_key = "test-key-123"

[[services]]
name = "ts.service"
display_name = "TeamSpeak"

[[services]]
name = "botDieta.service"
display_name = "Bot Dieta"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.poll_interval_secs, 30);
        assert_eq!(config.history_hours, 24);
        assert_eq!(config.api_key, "test-key-123");
        assert_eq!(config.services.len(), 2);
        assert_eq!(config.services[0].name, "ts.service");
        assert_eq!(config.services[0].display_name, "TeamSpeak");
        assert_eq!(config.services[1].name, "botDieta.service");
        assert_eq!(config.services[1].display_name, "Bot Dieta");
    }

    #[test]
    fn test_parse_empty_services() {
        let toml_str = r#"
poll_interval_secs = 10
history_hours = 6
api_key = "key"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.services.is_empty());
    }

    #[test]
    fn test_load_missing_file() {
        let result = load("/tmp/__nonexistent_config_123.toml");
        assert!(result.is_err());
    }
}
