pub mod gucs;
pub mod logs;
pub mod telemetry;

// In testing, this package gets installed as a Postgres extension.
// Behind cfg(test), we can initialize everything necessary to run `cargo pgrx test`.
// None of this will be compiled when using this as a shared library.

#[cfg(any(test, feature = "pg_test"))]
use pgrx::*;

#[cfg(any(test, feature = "pg_test"))]
pgrx::pg_module_magic!();

#[cfg(any(test, feature = "pg_test"))]
#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    crate::gucs::init();
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
