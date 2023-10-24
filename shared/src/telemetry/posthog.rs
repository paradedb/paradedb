use pgrx::*;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
struct Config {
    telemetry_handled: Option<String>, // Option because it's from a file, not env
    telemetry: String,
    posthog_api_key: String,
    posthog_host: String,
    commit_sha: String,
}

impl Config {
    fn from_env() -> Option<Self> {
        let telemetry_handled = std::fs::read_to_string("/tmp/telemetry")
            .map(|content| content.trim().to_string())
            .ok();

        envy::from_env::<Config>().ok().map(|config| Config {
            telemetry_handled,
            ..config
        })
    }
}

#[cfg(feature = "telemetry")]
fn should_enable_telemetry() -> bool {
    match std::env::var("TELEMETRY") {
        Ok(val) if val.to_lowercase() == "false" => false, // Explicitly turned off by env var
        Ok(_) | Err(_) => true, // Default to true if feature is enabled and env var is not "false"
    }
}

#[cfg(not(feature = "telemetry"))]
fn should_enable_telemetry() -> bool {
    false // Always false if feature is not enabled
}

pub fn init(event_name: &str) {
    if !should_enable_telemetry() {
        return;
    }

    if let Some(config) = Config::from_env() {
        if config.telemetry_handled.as_deref() == Some("true") || config.telemetry != "true" {
            return;
        }

        let endpoint = format!("{}/capture", config.posthog_host);
        let data = json!({
            "api_key": config.posthog_api_key,
            "event": event_name,
            "distinct_id": uuid::Uuid::new_v4().to_string(),
            "properties": {
                "commit_sha": config.commit_sha
            }
        });

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .body(data.to_string())
            .send();

        if let Err(e) = response {
            info!("Error sending request: {}", e);
        }
    }
}
