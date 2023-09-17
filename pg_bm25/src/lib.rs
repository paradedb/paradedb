use pgrx::*;
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
    let client = posthog_rs::client("phc_KiWfPSoxQLmFxY5yOODDBzzP3EcyPbn9oSVtsCBbasj");
    let event = Event::new("user signed up", "distinct_id_of_the_user");
    client.capture(event).unwrap();
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
