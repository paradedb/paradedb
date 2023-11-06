use pgrx::*;
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;

mod api;
mod index_access;
mod json;
mod manager;
mod operator;
mod parade_index;
mod tokenizers;

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_bm25");

pgrx::pg_module_magic!();

extension_sql_file!("../sql/_bootstrap.sql", bootstrap);
extension_sql_file!("../sql/_bootstrap_quickstart.sql");

// Initializes option parsing and telemetry
#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    index_access::options::init();
    telemetry::posthog::init("pg_bm25 Deployment");
    PARADE_LOGS_GLOBAL.init();
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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    #[pgrx::pg_test]
    fn test_parade_logs() {
        shared::test_plog!("pg_bm25");
    }
}
