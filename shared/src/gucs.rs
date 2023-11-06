use pgrx::{GucContext, GucFlags, GucRegistry, GucSetting};

pub fn init_logs(extension_name: &str, logs_var: &GucSetting<bool>) {
    // Define per-extension logs variable.
    GucRegistry::define_bool_guc(
        &format!("paradedb.{extension_name}.logs"),
        &format!("Enable logging to the paradedb.{extension_name}.logs table?"),
        "This incurs some overhead, so only recommended when debugging.",
        logs_var,
        GucContext::Userset,
        GucFlags::default(),
    );
}
