use super::connection;
use pgrx::*;

#[pg_extern]
pub fn load_iceberg() {
    connection::execute("INSTALL iceberg", []).expect("Failed to install iceberg");
    connection::execute("LOAD iceberg", []).expect("Failed to load iceberg");
}
