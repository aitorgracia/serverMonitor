use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceInfo {
    pub name:         String,
    pub display_name: String,
    pub running:      bool,
    pub cpu_usage:    f32,
    pub memory_mb:    u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Snapshot {
    pub timestamp:    i64,
    pub cpu_total:    f32,
    pub ram_used_gb:  f32,
    pub ram_total_gb: f32,
    pub services:     Vec<ServiceInfo>,
}

pub struct AgentClient {
    client:   Client,
    base_url: String,
    api_key:  String,
}

impl AgentClient {
    pub fn new(port: u16, api_key: &str) -> Self {
        Self {
            client:   Client::new(),
            base_url: format!("http://127.0.0.1:{}", port),
            api_key:  api_key.to_string(),
        }
    }

    pub async fn current(&self) -> Result<Snapshot, reqwest::Error> {
        self.client
            .get(format!("{}/metrics", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?
            .json::<Snapshot>()
            .await
    }

    pub async fn history(&self, hours: u64) -> Result<Vec<Snapshot>, reqwest::Error> {
        self.client
            .get(format!("{}/metrics/history", self.base_url))
            .query(&[("hours", hours)])
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?
            .json::<Vec<Snapshot>>()
            .await
    }
}
