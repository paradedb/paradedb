use pgrx::*;
use shared::logs::ParadeLogsGlobal;

mod api;
mod index_access;
mod sparse_index;

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_sparse");

pgrx::pg_module_magic!();

#[allow(clippy::missing_safety_doc)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    // Initializes option parsing
    index_access::options::init();

    PARADE_LOGS_GLOBAL.init();
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    #[pgrx::pg_test]
    fn test_parade_logs() {
        shared::test_plog!("pg_sparse");
    }
}
