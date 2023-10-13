use pgrx::*;
use serde_json::json;
use std::env;
use std::fs;

pub unsafe fn init() {
    // Check if telemetry was already handled at the ParadeDB level
    let telemetry_handled = fs::read_to_string("/tmp/telemetry")
        .map_or_else(|_| "false".to_string(), |content| content.trim().to_string());

    if telemetry_handled == "true" {
        return;
    }

    // Check if telemetry is enabled
    let telemetry = env::var("TELEMETRY").unwrap_or_else(|_| "false".to_string());
    if telemetry != "true" {
        info!("Telemetry for pg_search disabled!");
        return;
    }

    // Retrieve necessary environment variables or exit early if they're not set
    let api_key = match env::var("POSTHOG_API_KEY") {
        Ok(key) => key,
        Err(_) => return,
    };

    let posthog_host = match env::var("POSTHOG_HOST") {
        Ok(host) => host,
        Err(_) => return,
    };

    let commit_sha = env::var("COMMIT_SHA").unwrap_or_else(|_| "".to_string());

    // Construct the endpoint and event data
    let endpoint = format!("{}/capture", posthog_host);
    let data = json!({
        "api_key": api_key,
        "event": "pg_search Deployment",
        "distinct_id": uuid::Uuid::new_v4().to_string(),
        "properties": {
            "commit_sha": commit_sha
        }
    });

    // Send the event
    let client = reqwest::blocking::Client::new();
    let _response = client
        .post(endpoint)
        .header("Content-Type", "application/json")
        .body(data.to_string())
        .send();
}
