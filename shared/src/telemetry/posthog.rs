use anyhow::Result;
use serde_json::json;

use super::{event::TelemetryEvent, TelemetryConfigStore, TelemetryConnection, TelemetryStore};

struct PosthogConnection {
    api_key: String,
    host: String,
    client: reqwest::blocking::Client,
}

impl PosthogConnection {
    fn new(api_key: &str, host: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            host: host.to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/capture", self.host)
    }
}

pub struct PosthogStore {
    pub config_store: Box<dyn TelemetryConfigStore>,
}

impl TelemetryStore for PosthogStore {
    fn get_connection(&self) -> Result<Box<dyn TelemetryConnection>> {
        Ok(Box::new(PosthogConnection::new(
            &self.config_store.telemetry_api_key()?,
            &self.config_store.telemetry_host_url()?,
        )))
    }
}

impl TelemetryConnection for PosthogConnection {
    fn send(&self, uuid: &str, event: &TelemetryEvent) -> Result<()> {
        let mut properties = serde_json::to_value(event)?;
        properties["commit_sha"] = serde_json::to_value(event.commit_sha())?;

        let data = json!({
            "api_key": self.api_key,
            "event": event.name(),
            "distinct_id": uuid,
            "properties": properties,
        });

        self.client
            .post(self.endpoint())
            .header("Content-Type", "application/json")
            .body(data.to_string())
            .send()?;

        Ok(())
    }
}
