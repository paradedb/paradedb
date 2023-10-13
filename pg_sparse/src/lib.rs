use pgrx::prelude::*;

mod api;
mod index_access;
mod sparse_index;

pgrx::pg_module_magic!();

extension_sql_file!("../sql/_bootstrap_quickstart.sql");

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
