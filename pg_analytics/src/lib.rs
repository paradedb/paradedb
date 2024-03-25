#![allow(non_snake_case)]

mod datafusion;
mod errors;
mod federation;
mod guc;
mod hooks;
mod tableam;
mod types;

use crate::hooks::ParadeHook;
use guc::PostgresPgAnalyticsGucSettings;
use pgrx::*;
use shared::telemetry::{setup_telemetry_background_worker, ParadeExtension};

pgrx::pg_module_magic!();
extension_sql_file!("../sql/_bootstrap.sql");

// A static variable is required to host grand unified configuration settings.
pub static GUCS: PostgresPgAnalyticsGucSettings = PostgresPgAnalyticsGucSettings::new();
// These are the hooks that we register with Postgres.
static mut PARADE_HOOK: ParadeHook = ParadeHook;

#[pg_guard]
pub extern "C" fn _PG_init() {
    GUCS.init("pg_analytics");

    #[allow(static_mut_ref)]
    unsafe {
        register_hook(&mut PARADE_HOOK)
    };

    setup_telemetry_background_worker(ParadeExtension::PgAnalytics);
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
