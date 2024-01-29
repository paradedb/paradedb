#![allow(non_snake_case)]

mod api;
mod datafusion;
mod errors;
mod guc;
mod hooks;
mod tableam;

use pgrx::*;
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;

use crate::guc::PARADE_GUC;
use crate::hooks::ParadeHook;

pgrx::pg_module_magic!();
extension_sql_file!("../sql/_bootstrap.sql");

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_analytics");
// These are the hooks that we register with Postgres.
static mut PARADE_HOOK: ParadeHook = ParadeHook;

#[pg_guard]
pub extern "C" fn _PG_init() {
    telemetry::posthog::init("pg_analytics");
    PARADE_LOGS_GLOBAL.init();
    PARADE_GUC.init();

    #[allow(static_mut_ref)]
    unsafe {
        register_hook(&mut PARADE_HOOK)
    };
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
