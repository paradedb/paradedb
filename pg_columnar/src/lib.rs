#![allow(non_snake_case)]

mod api;
mod datafusion;
mod hooks;
mod tableam;

use pgrx::*;
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;

use crate::hooks::ParadeHook;

pgrx::pg_module_magic!();
extension_sql_file!("../sql/_bootstrap.sql");

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_columnar");
static mut PARADE_HOOK: ParadeHook = ParadeHook;

#[pg_guard]
pub extern "C" fn _PG_init() {
    telemetry::posthog::init("pg_columnar deployment");
    PARADE_LOGS_GLOBAL.init();

    unsafe { register_hook(&mut PARADE_HOOK) };
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
