use pgrx::prelude::*;
use posthog_rs::Event;
use std::env;
use uuid::Uuid;

mod api;

pgrx::pg_module_magic!();

extension_sql_file!("../sql/_bootstrap_quickstart.sql");

#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    let telemetry = env::var("TELEMETRY").unwrap_or_else(|_| String::from("True"));
    let telemetry_sent = env::var("TELEMETRY_SENT").unwrap_or_else(|_| String::from("False"));
    if telemetry == "False" {
        println!("Telemetry is disabled.");
    } else if telemetry_sent != "True" {
        if let Ok(api_key) = env::var("POSTHOG_API_KEY") {
            let client = posthog_rs::client(api_key.as_str());
            let mut event = Event::new("pg_bm25 Deployment", &Uuid::new_v4().to_string());
            if let Ok(commit_sha) = env::var("COMMIT_SHA") {
                event.insert_prop("commit_sha", &commit_sha).unwrap();
            } else {
                eprintln!("Failed to retrieve COMMIT_SHA from environment variables, sending telemetry without commit_sha!");
            }
            client.capture(event).unwrap();
        } else {
            eprintln!("Failed to retrieve POSTHOG_API_KEY from environment variables, not sending telemetry!");
        }
    }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
