use pgrx::*;
use std::fs;
use std::path::PathBuf;

#[pg_extern]
pub fn create_bm25_test_table() {
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sql_file_path = base_path.join("sql/_bootstrap_quickstart.sql");
    let file_content = fs::read_to_string(sql_file_path).expect("Error reading SQL file");
    let _ = Spi::run_with_args(&file_content, None);
}
