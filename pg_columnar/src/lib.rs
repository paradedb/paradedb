use pgrx::prelude::*;
use pgrx::{pg_sys::TableAmRoutine, pg_sys::NodeTag};
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;

pgrx::pg_module_magic!();

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_columnar");

const memam_methods: pg_sys::TableAmRoutine = pg_sys::TableAmRoutine {
    type_: pg_sys::NodeTag::T_TableAmRoutine
};

// #[pg_extern]
// pub fn mem_tableam_handler() -> *const pg_sys::TableAmRoutine {
//     return &memam_methods;
// }

// initializes telemetry
#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    telemetry::posthog::init("pg_columnar Deployment");
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
        shared::test_plog!("pg_columnar");
    }
}
