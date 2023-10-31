use pgrx::*;

mod api;
mod index_access;
mod sparse_index;

pgrx::pg_module_magic!();

// Initializes option parsing
#[allow(clippy::missing_safety_doc)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    index_access::options::init();
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
