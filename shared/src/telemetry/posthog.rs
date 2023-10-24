use pgrx::*;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
struct Config {
    telemetry_handled: Option<String>, // Option because it won't be set if running the extension standalone
    telemetry: Option<String>, // Option because it won't be set if telemetry is disabled
    posthog_api_key: String,
    posthog_host: String,
    commit_sha: String,
}

impl Config {
    fn from_env() -> Option<Self> {
        let telemetry_handled = std::fs::read_to_string("/tmp/telemetry")
            .map(|content| content.trim().to_string())
            .ok();

        #[cfg(feature = "telemetry")]
        let default_telemetry = Some("true".to_string());

        #[cfg(not(feature = "telemetry"))]
        let default_telemetry = Some("false".to_string());
    
        let telemetry = Some(std::env::var("TELEMETRY").unwrap_or(default_telemetry.unwrap()));

        envy::from_env::<Config>().ok().map(|config| Config {
            telemetry_handled,
            telemetry,
            ..config
        })
    }
}

pub fn init(event_name: &str) {
    info!("hello");


    // tested, it does send if --features telemetry and doesn't send if that's not provided
    // only need to test if can be overwritten by env var now, and it'll be done
    // perfect, it can be overwritten. works well!

    
    if let Some(config) = Config::from_env() {
        info!("Telemetry config: {:?}", config);

        if config.telemetry != Some("true".to_string()) || config.telemetry_handled == Some("true".to_string()) {
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
