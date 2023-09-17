use pgrx::*;
use std::env;
use posthog_rs::Event;

mod api;
mod index_access;
mod json;
mod manager;
mod operator;
mod parade_index;

pgrx::pg_module_magic!();

extension_sql_file!("../sql/_bootstrap.sql", bootstrap);
extension_sql_file!("../sql/_bootstrap_quickstart.sql");

#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    // Retrieve the API key from the environment variable
    if let Ok(api_key) = env::var("POSTHOG_API_KEY") {
        let client = posthog_rs::client(api_key.as_str());
        let mut event = Event::new("user signed up", "distinct_id_of_the_user");
        
        if let Ok(commit_sha) = env::var("COMMIT_SHA") {
            event.insert_prop("commit_sha", &commit_sha).unwrap();
        } else {
            eprintln!("Failed to retrieve COMMIT_SHA from environment variables!");
        }

        client.capture(event).unwrap();
    } else {
        eprintln!("Failed to retrieve POSTHOG_API_KEY from environment variables!");
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
