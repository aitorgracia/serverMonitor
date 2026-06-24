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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example_config() {
        let toml_str = r#"
ssh_host      = "monitor@192.168.1.100"
ssh_key       = "~/.ssh/id_dashboard"
api_key       = "test-key-456"
local_port    = 3000
refresh_secs  = 5
history_hours = 6
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ssh_host, "monitor@192.168.1.100");
        assert_eq!(config.ssh_key, "~/.ssh/id_dashboard");
        assert_eq!(config.api_key, "test-key-456");
        assert_eq!(config.local_port, 3000);
        assert_eq!(config.refresh_secs, 5);
        assert_eq!(config.history_hours, 6);
    }

    #[test]
    fn test_load_missing_file() {
        let result = load("/tmp/__nonexistent_dashboard_config.toml");
        assert!(result.is_err());
    }
}
