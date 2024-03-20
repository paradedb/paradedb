use serde_json::json;

use super::{event::TelemetryEvent, TelemetryConnection, TelemetryError, TelemetryStore};

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
    api_key: String,
    host: String,
}

impl PosthogStore {
    pub fn from_env() -> Result<Self, TelemetryError> {
        Ok(Self {
            api_key: option_env!("POSTHOG_API_KEY")
                .map(|s| s.to_string())
                .ok_or(TelemetryError::PosthogApiKey)?,
            host: option_env!("POSTHOG_HOST")
                .map(|s| s.to_string())
                .ok_or(TelemetryError::PosthogHost)?,
        })
    }
}

impl TelemetryStore for PosthogStore {
    type Error = TelemetryError;

    fn get_connection(
        &self,
    ) -> Result<Box<dyn TelemetryConnection<Error = Self::Error>>, Self::Error> {
        Ok(Box::new(PosthogConnection::new(&self.api_key, &self.host)))
    }
}

impl TelemetryConnection for PosthogConnection {
    type Error = TelemetryError;

    fn send(&self, uuid: &str, event: &TelemetryEvent) -> Result<(), Self::Error> {
        let data = json!({
            "api_key": self.api_key,
            "event": event.name(),
            "distinct_id": uuid,
            "properties": {
                "commit_sha": event.commit_sha(),
                "telemetry_data": event.to_json(),
            },
        });
        if event.enabled() {
            self.client
                .post(self.endpoint())
                .header("Content-Type", "application/json")
                .body(data.to_string())
                .send()
                .map(|_| ())
                .map_err(TelemetryError::from)
        } else {
            pgrx::log!(
                "paradedb telemetry is disabled, not sending event: {}",
                event.name()
            );
            Ok(())
        }
    }
}
