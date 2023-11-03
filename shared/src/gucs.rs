use pgrx::{GucContext, GucFlags, GucRegistry, GucSetting};

pub static PARADEDB_LOGS: GucSetting<bool> = GucSetting::<bool>::new(false);

pub fn init() {
    GucRegistry::define_bool_guc(
        "paradedb.logs",
        "Enable logging to the paradedb.logs table?",
        "This incurs some overhead, so only recommended when debugging.",
        &PARADEDB_LOGS,
        GucContext::Userset,
        GucFlags::default(),
    );
}
