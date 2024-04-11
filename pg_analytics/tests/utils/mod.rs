use shared::fixtures::db::Query;
use sqlx::PgConnection;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn test_data_path(conn: &mut PgConnection) -> PathBuf {
    let db_name = "SELECT current_database()".fetch_one::<(String,)>(conn).0;
    let data_dir = "SHOW data_directory".fetch_one::<(String,)>(conn).0;
    let parade_dir = "deltalake";
    let db_oid = format!("SELECT oid FROM pg_database WHERE datname='{db_name}'")
        .fetch_one::<(sqlx::postgres::types::Oid,)>(conn)
        .0
         .0;

    PathBuf::from(&data_dir)
        .join(parade_dir)
        .join(db_oid.to_string())
}

pub fn path_is_parquet_file(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => ext == "parquet",
        None => false,
    }
}

pub fn total_parquet_files_in_dir(path: &Path) -> usize {
    WalkDir::new(path)
        .into_iter()
        .filter(|e| path_is_parquet_file(e.as_ref().unwrap().path()))
        .count()
}
