use pgrx::*;
use shared::telemetry;

mod api;
mod index_access;
mod json;
mod manager;
mod operator;
mod parade_index;
mod tokenizers;

pgrx::pg_module_magic!();

extension_sql_file!("../sql/_bootstrap.sql", bootstrap);
extension_sql_file!("../sql/_bootstrap_quickstart.sql");

// initializes option parsing and telemetry
#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    index_access::options::init();
    telemetry::posthog::init("pg_bm25 Deployment");
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
