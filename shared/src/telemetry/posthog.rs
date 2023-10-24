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

        info!("hi there!");

        #[cfg(feature = "telemetry")]
        let default_telemetry = true.to_string();

        #[cfg(not(feature = "telemetry"))]
        let default_telemetry = false.to_string();

        info!("default_telemetry: {:?}", default_telemetry);

        let telemetry = std::env::var("TELEMETRY").unwrap_or(default_telemetry);

        // okay, now it properly sets telemetry to true/false based on whether the --features telemetry is enabled!

        info!("oooooo");
        info!("telemetry: {:?}", telemetry);
        info!("telemetry_handled: {:?}", telemetry_handled);

        envy::from_env::<Config>().ok().map(|config| Config {
            telemetry_handled,
            telemetry,
            ..config
        })
    }
}

pub fn init(event_name: &str) {
    info!("hello");

    if let Some(config) = Config::from_env() {
        info!("Telemetry config: {:?}", config);

        if config.telemetry != "true" || config.telemetry_handled == Some("true".to_string()) {
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
