use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

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

    async fn send_and_check(&self, req: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
        let resp = req.send().await.map_err(|e| format!("Error de conexión: {}", e))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {} — {}", status, body));
        }
        Ok(resp)
    }

    pub async fn current(&self) -> Result<Snapshot, String> {
        let resp = self.send_and_check(
            self.client
                .get(format!("{}/metrics", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
        ).await?;
        resp.json::<Snapshot>().await
            .map_err(|e| format!("Error decodificando JSON en /metrics: {}", e))
    }

    pub async fn history(&self, hours: u64) -> Result<Vec<Snapshot>, String> {
        let resp = self.send_and_check(
            self.client
                .get(format!("{}/metrics/history", self.base_url))
                .query(&[("hours", hours)])
                .header("Authorization", format!("Bearer {}", self.api_key))
        ).await?;
        resp.json::<Vec<Snapshot>>().await
            .map_err(|e| format!("Error decodificando JSON en /metrics/history: {}", e))
    }

    pub async fn start_service(&self, name: &str) -> Result<String, String> {
        let resp = self.send_and_check(
            self.client
                .post(format!("{}/services/{}/start", self.base_url, name))
                .header("Authorization", format!("Bearer {}", self.api_key))
        ).await?;
        let json: Value = resp.json().await
            .map_err(|e| format!("Error decodificando JSON en start_service: {}", e))?;
        Ok(json["message"].as_str().unwrap_or("ok").to_string())
    }

    pub async fn stop_service(&self, name: &str) -> Result<String, String> {
        let resp = self.send_and_check(
            self.client
                .post(format!("{}/services/{}/stop", self.base_url, name))
                .header("Authorization", format!("Bearer {}", self.api_key))
        ).await?;
        let json: Value = resp.json().await
            .map_err(|e| format!("Error decodificando JSON en stop_service: {}", e))?;
        Ok(json["message"].as_str().unwrap_or("ok").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_snapshot() {
        let json = r#"
        {
            "timestamp": 1719000000,
            "cpu_total": 45.2,
            "ram_used_gb": 4.5,
            "ram_total_gb": 16.0,
            "services": [
                {
                    "name": "ts.service",
                    "display_name": "TeamSpeak",
                    "running": true,
                    "cpu_usage": 2.3,
                    "memory_mb": 128
                },
                {
                    "name": "bot.service",
                    "display_name": "Bot",
                    "running": false,
                    "cpu_usage": 0.0,
                    "memory_mb": 0
                }
            ]
        }
        "#;
        let snapshot: Snapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snapshot.timestamp, 1719000000);
        assert_eq!(snapshot.cpu_total, 45.2);
        assert_eq!(snapshot.ram_used_gb, 4.5);
        assert_eq!(snapshot.ram_total_gb, 16.0);
        assert_eq!(snapshot.services.len(), 2);
        assert!(snapshot.services[0].running);
        assert!(!snapshot.services[1].running);
        assert_eq!(snapshot.services[0].name, "ts.service");
        assert_eq!(snapshot.services[1].display_name, "Bot");
    }

    #[test]
    fn test_deserialize_service_info() {
        let json = r#"
        {
            "name": "test.service",
            "display_name": "Test",
            "running": true,
            "cpu_usage": 5.1,
            "memory_mb": 256
        }
        "#;
        let svc: ServiceInfo = serde_json::from_str(json).unwrap();
        assert_eq!(svc.name, "test.service");
        assert!(svc.running);
        assert_eq!(svc.cpu_usage, 5.1);
        assert_eq!(svc.memory_mb, 256);
    }
}
