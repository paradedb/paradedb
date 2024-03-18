use crate::telemetry::data::read_telemetry_data;

use super::data::Directory;
use super::TelemetryError;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub id: String,
    pub extension_name: String,
    pub commit_sha: Option<String>, // Don't block sending telemetry if COMMIT_SHA is unset
    pub posthog_api_key: String,
    pub posthog_host: String,
    pub enabled: bool,
}

impl Config {
    fn new(extension_name: &str) -> Result<Self, TelemetryError> {
        Ok(Self {
            id: Self::id(extension_name)?,
            extension_name: extension_name.to_string(),
            commit_sha: std::env::var("COMMIT_SHA").ok(),
            posthog_api_key: std::env::var("POSTHOG_API_KEY")
                .map_err(|_| TelemetryError::PosthogApiKey)?,
            posthog_host: std::env::var("POSTHOG_HOST").map_err(|_| TelemetryError::PosthogHost)?,
            enabled: std::env::var("PARADEDB_TELEMETRY")
                .map(|e| e.to_lowercase().trim() == "true")
                .unwrap_or(cfg!(telemetry)),
        })
    }

    fn id(extension_name: &str) -> Result<String, TelemetryError> {
        let uuid_file =
            Directory::extension(extension_name)?.join(format!("{extension_name}_uuid"));

        match fs::read_to_string(&uuid_file)
            .map_err(TelemetryError::ReadUuid)
            .and_then(|s| Uuid::parse_str(&s).map_err(TelemetryError::ParseUuid))
        {
            Ok(uuid) => Ok(uuid.to_string()),
            _ => {
                let new_uuid = Uuid::new_v4().to_string();
                fs::write(&uuid_file, &new_uuid).expect("Unable to write UUID to file");
                Ok(new_uuid)
            }
        }
    }

    fn handled() -> Result<bool, TelemetryError> {
        std::fs::read_to_string("/tmp/telemetry")
            .map(|content| content.to_lowercase().trim() == "true")
            .map_err(TelemetryError::HandledCheck)
    }
}

pub trait HttpClient {
    fn post(&self, url: String, body: String) -> Result<(), TelemetryError>;
}

// Implement the trait for `reqwest::blocking::Client`
impl HttpClient for reqwest::blocking::Client {
    fn post(&self, url: String, body: String) -> Result<(), TelemetryError> {
        self.post(url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .map(|_| ())
            .map_err(TelemetryError::from)
    }
}

pub struct PosthogClient<C: HttpClient> {
    client: C,
    config: Config,
}

impl<C: HttpClient> PosthogClient<C> {
    pub fn new(client: C, config: Config) -> Self {
        Self { client, config }
    }

    fn send(&self, event: &str, properties: Option<Value>) -> Result<(), TelemetryError> {
        if Config::handled()? || !self.config.enabled {
            return Ok(());
        }

        let endpoint = format!("{}/capture", self.config.posthog_host);
        let data = json!({
            "api_key": self.config.posthog_api_key,
            "event": event,
            "distinct_id": self.config.id,
            "properties": properties,
        });

        self.client.post(endpoint, data.to_string())
    }

    pub fn send_deployment(&self) -> Result<(), TelemetryError> {
        let event = format!("{} Deployment", self.config.extension_name);
        let properties = json!({
            "commit_sha": self.config.commit_sha
        });
        self.send(&event, Some(properties))
    }

    pub fn send_directory_data(&self) -> Result<(), TelemetryError> {
        let event = format!("{} Directory Data", self.config.extension_name);
        let properties = json!({
            "commit_sha": self.config.commit_sha,
            "telemetry_data": read_telemetry_data(&self.config.extension_name)?
        });
        self.send(&event, Some(properties))
    }
}

impl PosthogClient<reqwest::blocking::Client> {
    pub fn from_extension_name(extension_name: &str) -> Result<Self, TelemetryError> {
        let config = Config::new(extension_name)?;
        let client = reqwest::blocking::Client::new();
        Ok(Self::new(client, config))
    }
}
