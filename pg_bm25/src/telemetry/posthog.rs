use pgrx::*;
use serde::Deserialize;
use serde_json::json;
use std::fmt;

#[derive(Deserialize, Debug)]
struct Config {
    telemetry_handled: Option<String>, // Option because it's from a file, not env
    telemetry: String,
    posthog_api_key: String,
    posthog_host: String,
    commit_sha: String,
}

enum EnvError {
    PosthogApiKey,
    PosthogHost,
    CommitSha,
    ParsingError,
}

impl fmt::Display for EnvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EnvError::PosthogApiKey => write!(f, "Failed to retrieve POSTHOG_API_KEY from environment variables, not sending pg_bm25 telemetry!"),
            EnvError::PosthogHost => write!(f, "Failed to retrieve POSTHOG_HOST from environment variables, not sending pg_bm25 telemetry!"),
            EnvError::CommitSha => write!(f, "Failed to retrieve COMMIT_SHA from environment variables, sending pg_bm25 telemetry without commit_sha!"),
            EnvError::ParsingError => write!(f, "Telemetry for pg_bm25 disabled!"),
        }
    }
}

impl Config {
    fn from_env() -> Result<Self, EnvError> {
        let telemetry_handled = std::fs::read_to_string("/tmp/telemetry")
            .map(|content| content.trim().to_string())
            .ok();

        match envy::from_env::<Config>() {
            Ok(config) => Ok(Config {
                telemetry_handled,
                ..config
            }),
            Err(envy::Error::MissingValue(field)) => match field {
                "posthog_api_key" => Err(EnvError::PosthogApiKey),
                "posthog_host" => Err(EnvError::PosthogHost),
                "commit_sha" => Err(EnvError::CommitSha),
                _ => Err(EnvError::ParsingError),
            },
            Err(_) => Err(EnvError::ParsingError),
        }
    }
}

pub fn init() {
    let config = Config::from_env();

    match config {
        Ok(config) => {
            if config.telemetry_handled.as_deref() == Some("true") || config.telemetry != "true" {
                return;
            }

            let endpoint = format!("{}/capture", config.posthog_host);
            let data = json!({
                "api_key": config.posthog_api_key,
                "event": "pg_bm25 Deployment",
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

            match response {
                Ok(res) if res.status().is_success() => {
                    let _body = res
                        .text()
                        .unwrap_or_else(|_| String::from("Failed to read response body"));
                }
                Ok(res) => {
                    info!("Failed to send event. Status: {}", res.status());
                }
                Err(e) => {
                    info!("Error sending request: {}", e);
                }
            }
        }
        Err(e) => {
            info!("{}", e);
        }
    }
}
