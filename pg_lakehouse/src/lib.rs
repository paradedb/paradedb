mod api;
mod datafusion;
mod fdw;
mod hooks;
mod schema;

use hooks::LakehouseHook;
use pgrx::*;
// use shared::telemetry::{setup_telemetry_background_worker, ParadeExtension};

pg_module_magic!();

static mut EXTENSION_HOOK: LakehouseHook = LakehouseHook;

#[pg_guard]
pub extern "C" fn _PG_init() {
    #[allow(static_mut_refs)]
    unsafe {
        register_hook(&mut EXTENSION_HOOK)
    };

    // TODO: Re-enable once we reconfigure telemetry to not write to the file system
    // setup_telemetry_background_worker(ParadeExtension::PgLakehouse);
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
