use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::Path;

use crate::telemetry::data::{get_postgres_data_directory, read_telemetry_data};

#[derive(Deserialize, Debug)]
struct Config {
    telemetry_handled: Option<String>, // This won't be set if running the extension standalone
    telemetry: Option<String>,         // This won't be set if PARADEDB_TELEMETRY is disabled
    commit_sha: Option<String>,        // Don't block sending telemetry if COMMIT_SHA is unset
    posthog_api_key: String,
    posthog_host: String,
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

        let telemetry =
            Some(std::env::var("PARADEDB_TELEMETRY").unwrap_or(default_telemetry.to_string()));

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

        // Retrieve the PostgreSQL data directory
        let pg_data_directory = match get_postgres_data_directory() {
            Some(dir) => dir,
            None => {
                eprintln!("PGDATA environment variable is not set");
                return; // Early return from the function
            }
        };

        // Construct the uuid_file path using the dynamically retrieved PGDATA path
        // For privacy reasons, we generate an anonymous UUID for each new deployment
        let uuid_file = format!("/bitnami/postgresql/data/{}_uuid", extension_name);

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
    // The PARADEDB_TELEMETRY environment variable controls both functions, allowing it to be used
    // for opting out of all telemetry.
    if let Some(config) = Config::from_env() {
        if config.telemetry.as_deref() == Some("true") {
            let uuid_dir = "/bitnami/postgresql/data";
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

// A structure to hold the necessary configuration for sending telemetry
struct TelemetrySettings {
    distinct_id: String,
    posthog_host: String,
    posthog_api_key: String,
    commit_sha: String,
}

fn initialize_telemetry(extension_name: String) -> Option<TelemetrySettings> {
    let config = Config::from_env();
    pgrx::log!("config: {:?}", config);

    if let Some(config) = config {
        if config.telemetry.as_deref() != Some("true")
            || config.telemetry_handled.as_deref() == Some("true")
        {
            return None;
        }

        let pg_data_directory = get_postgres_data_directory()?;
        let uuid_file = format!("{}/{}_uuid", pg_data_directory, extension_name);

        let distinct_id = if Path::new(&uuid_file).exists() {
            fs::read_to_string(&uuid_file)
                .ok()
                .and_then(|uuid_str| {
                    uuid::Uuid::parse_str(&uuid_str)
                        .ok()
                        .map(|uuid| uuid.to_string())
                })
                .unwrap_or_else(|| generate_and_save_uuid(&uuid_file))
        } else {
            generate_and_save_uuid(&uuid_file)
        };

        Some(TelemetrySettings {
            distinct_id,
            posthog_host: config.posthog_host,
            posthog_api_key: config.posthog_api_key,
            commit_sha: config.commit_sha?,
        })
    } else {
        None
    }

    // TODO: Make it send an init ping here to keep up with current deployment framework
}

fn generate_and_save_uuid(uuid_file: &str) -> String {
    let new_uuid = uuid::Uuid::new_v4().to_string();
    fs::write(uuid_file, &new_uuid).expect("Unable to write UUID to file");
    new_uuid
}

fn send_telemetry_data(settings: &TelemetrySettings, extension_name: &str) {
    let telemetry_data = read_telemetry_data(extension_name);
    pgrx::log!("telemetry data: {:?}", telemetry_data);

    let endpoint = format!("{}/capture", settings.posthog_host);
    let data = json!({
        "api_key": settings.posthog_api_key,
        "event": format!("{} Deployment", extension_name),
        "distinct_id": settings.distinct_id,
        "properties": {
            "commit_sha": settings.commit_sha,
            "telemetry_data": telemetry_data
        }
    });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(endpoint)
        .header("Content-Type", "application/json")
        .body(data.to_string())
        .send();

    if let Err(e) = response {
        eprintln!("Error sending request: {}", e);
    }
}
