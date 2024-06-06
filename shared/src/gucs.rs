use pgrx::{GucContext, GucFlags, GucRegistry, GucSetting};

pub trait GlobalGucSettings {
    fn telemetry_enabled(&self) -> bool;
}

pub struct PostgresGlobalGucSettings {
    telemetry: GucSetting<bool>,
}

impl PostgresGlobalGucSettings {
    pub const fn new() -> Self {
        Self {
            telemetry: GucSetting::<bool>::new(true),
        }
    }

    pub fn init(&self, extension_name: &str) {
        // Note that Postgres is very specific about the naming convention of variables.
        // They must be namespaced... we use 'paradedb.<variable>' below.
        // They cannot have more than one '.' - paradedb.pg_search.telemetry will not work.

        // telemetry
        GucRegistry::define_bool_guc(
            &format!("paradedb.{extension_name}_telemetry"),
            &format!("Enable telemetry on the ParadeDB {extension_name} extension.",),
            &format!("Enable telemetry on the ParadeDB {extension_name} extension.",),
            &self.telemetry,
            GucContext::Userset,
            GucFlags::default(),
        );
    }
}

impl Default for PostgresGlobalGucSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalGucSettings for PostgresGlobalGucSettings {
    fn telemetry_enabled(&self) -> bool {
        // If PARADEDB_TELEMETRY is not 'true' at compile time, then we will never enable.
        // This is useful for test builds and CI.
        option_env!("PARADEDB_TELEMETRY") == Some("true") && self.telemetry.get()
    }
}
