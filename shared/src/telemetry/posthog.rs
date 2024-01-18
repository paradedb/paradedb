use pgrx::*;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::Path;

use crate::constants::{PARADEDB_NAME, PG_BM25_NAME};

#[derive(Deserialize, Debug)]
struct Config {
    telemetry_handled: Option<String>, // Option because it won't be set if running the extension standalone
    telemetry: Option<String>,         // Option because it won't be set if telemetry is disabled
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
        let default_telemetry = "true";

        #[cfg(not(feature = "telemetry"))]
        let default_telemetry = "false";

        let telemetry = Some(std::env::var("TELEMETRY").unwrap_or(default_telemetry.to_string()));

        envy::from_env::<Config>().ok().map(|config| Config {
            telemetry_handled,
            telemetry,
            ..config
        })
    }
}

pub fn init(extension_name: &str) {
    if let Some(config) = Config::from_env() {
        // Exit early if telemetry is not enabled or has already been handled
        if config.telemetry.as_deref() != Some("true")
            || config.telemetry_handled.as_deref() == Some("true")
        {
            return;
        }

        // For privacy reasons, we generate an anonymous UUID for each new deployment
        let uuid_file = format!("/var/lib/postgresql/data/{}_uuid", extension_name);

        // Closure to generate a new UUID and write it to the file
        let generate_and_save_uuid = || {
            let new_uuid = uuid::Uuid::new_v4().to_string();
            fs::write(&uuid_file, &new_uuid).expect("Unable to write UUID to file");
            new_uuid
        };

        let distinct_id = if Path::new(&uuid_file).exists() {
            match fs::read_to_string(&uuid_file) {
                Ok(uuid_str) => match uuid::Uuid::parse_str(&uuid_str) {
                    Ok(uuid) => uuid.to_string(),
                    Err(_) => generate_and_save_uuid(),
                },
                Err(_) => generate_and_save_uuid(),
            }
        } else {
            generate_and_save_uuid()
        };

        let endpoint = format!("{}/capture", config.posthog_host);
        let data = json!({
            "api_key": config.posthog_api_key,
            "event": format!("{} Deployment", extension_name),
            "distinct_id": distinct_id,
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

pub fn connection_start() {
    // This function shares configuration with the `init` function on this file.
    // The TELEMETRY environment variable controls both functions, allowing it to be used
    // for opting out of all telemetry.
    if let Some(config) = Config::from_env() {
        if config.telemetry.as_deref() == Some("true") {
            let uuid_dir = "/var/lib/postgresql/data";
            let extension_name;
            let file_content = if Path::new(uuid_dir)
                .join(format!("{}_uuid", PARADEDB_NAME.to_lowercase()))
                .exists()
            {
                extension_name = PARADEDB_NAME;
                fs::read_to_string(
                    Path::new(uuid_dir).join(format!("{}_uuid", PARADEDB_NAME.to_lowercase())),
                )
            } else {
                extension_name = PG_BM25_NAME;
                fs::read_to_string(Path::new(uuid_dir).join(format!("{}_uuid", PG_BM25_NAME)))
            };

            let distinct_id = match file_content {
                Ok(uuid) => uuid,
                Err(_) => {
                    warning!("telemetry enabled but uuid file is empty!");
                    return;
                }
            };

            let endpoint = format!("{}/capture", config.posthog_host);
            let data = json!({
                "api_key": config.posthog_api_key,
                "event": format!("{} Connection Started", extension_name),
                "distinct_id": distinct_id,
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
                warning!("Error sending telemetry request: {}", e);
            }
        }
    }
}
