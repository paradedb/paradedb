use pgrx::*;
use serde_json::json;
use std::env;
use std::fs;

pub unsafe fn init() {
    let telemetry = env::var("TELEMETRY").unwrap_or_else(|_| String::from("True"));

    // Read TELEMETRY_SENT from a file
    let telemetry_sent = match fs::read_to_string("/tmp/telemetry_sent") {
        Ok(content) => content.trim().to_string(),
        Err(_) => String::from("False"),
    };

    if telemetry == "False" {
        info!("pg_bm25 telemetry disabled");
    } else if telemetry_sent != "True" {
        if let Ok(api_key) = env::var("POSTHOG_API_KEY") {
            if let Ok(posthog_host) = env::var("POSTHOG_HOST") {
                if let Ok(commit_sha) = env::var("COMMIT_SHA") {
                    // The endpoint for sending events to PostHog
                    let endpoint: String = format!("{}/capture", posthog_host);

                    // Define the event data
                    let data = json!({
                        "api_key": api_key,
                        "event": "pg_bm25 Deployment",
                        "distinct_id": uuid::Uuid::new_v4().to_string(),
                        "properties": {
                            "commit_sha": commit_sha
                        }
                    });

                    // Create a new HTTP client and send the event
                    let client = reqwest::blocking::Client::new();
                    let response = client
                        .post(endpoint)
                        .header("Content-Type", "application/json")
                        .body(data.to_string())
                        .send();

                    // Check if the request was successful
                    match response {
                        Ok(res) if res.status().is_success() => {
                            info!("Event sent successfully!");
                            let body = res
                                .text()
                                .unwrap_or_else(|_| String::from("Failed to read response body"));
                            info!("Response body: {}", body);
                        }
                        Ok(res) => {
                            info!("Failed to send event. Status: {}", res.status());
                        }
                        Err(e) => {
                            info!("Error sending request: {}", e);
                        }
                    }
                } else {
                    info!("Failed to retrieve COMMIT_SHA from environment variables, sending telemetry without commit_sha!");
                }
            } else {
                info!("Failed to retrieve POSTHOG_HOST from environment variables, not sending telemetry!");
            }
        } else {
            info!("Failed to retrieve POSTHOG_API_KEY from environment variables, not sending telemetry!");
        }
    }
}
