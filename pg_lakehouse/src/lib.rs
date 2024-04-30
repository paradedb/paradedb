mod cell;
mod datetime;
mod fdw;
mod format;
mod object_store;
mod options;
mod s3;
mod table;

use pgrx::*;

pg_module_magic!();

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
