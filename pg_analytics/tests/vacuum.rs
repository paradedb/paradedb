mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn test_data_path(mut conn: &mut PgConnection) -> PathBuf {
    let db_name = "SELECT current_database()"
        .fetch_one::<(String,)>(&mut conn)
        .0;
    let data_dir = "SHOW data_directory".fetch_one::<(String,)>(&mut conn).0;
    let parade_dir = "deltalake";
    let db_oid = format!("SELECT oid FROM pg_database WHERE datname='{db_name}'")
        .fetch_one::<(sqlx::postgres::types::Oid,)>(&mut conn)
        .0
         .0;

    PathBuf::from(&data_dir)
        .join(parade_dir)
        .join(db_oid.to_string())
}

fn path_is_parquet_file(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => ext == "parquet",
        None => false,
    }
}

fn total_files_in_dir(path: &PathBuf) -> usize {
    WalkDir::new(path.clone())
        .into_iter()
        .filter(|e| path_is_parquet_file(e.as_ref().unwrap().path()))
        .count()
}

#[rstest]
fn vacuum(mut conn: PgConnection) {
    "CREATE TABLE t (a int) USING parquet".execute(&mut conn);
    "CREATE TABLE s (a int)".execute(&mut conn);
    "INSERT INTO t VALUES (1), (2), (3)".execute(&mut conn);
    "INSERT INTO s VALUES (4), (5), (6)".execute(&mut conn);
    "VACUUM".execute(&mut conn);
    "VACUUM FULL".execute(&mut conn);
    "VACUUM t".execute(&mut conn);
    "VACUUM FULL t".execute(&mut conn);
    "DROP TABLE t, s".execute(&mut conn);
    "VACUUM".execute(&mut conn);
}

#[rstest]
fn vacuum_check_files(mut conn: PgConnection) {
    "CREATE TABLE t (a int) USING parquet".execute(&mut conn);
    "CREATE TABLE s (a int)".execute(&mut conn);
    "INSERT INTO t VALUES (1), (2), (3)".execute(&mut conn);
    "INSERT INTO s VALUES (4), (5), (6)".execute(&mut conn);

    let data_path = test_data_path(&mut conn);

    let total_pre_vacuum_files = total_files_in_dir(&data_path);

    "DROP TABLE t, s".execute(&mut conn);
    "VACUUM".execute(&mut conn);

    let total_post_vacuum_files = total_files_in_dir(&data_path);

    assert!(total_pre_vacuum_files > total_post_vacuum_files);
}

#[rstest]
fn vacuum_full_check_files(mut conn: PgConnection) {
    "CREATE TABLE t (a int) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1), (2), (3)".execute(&mut conn);
    "INSERT INTO t VALUES (4), (5), (6)".execute(&mut conn);

    let data_path = test_data_path(&mut conn);

    let total_pre_vacuum_files = total_files_in_dir(&data_path);

    "VACUUM FULL".execute(&mut conn);

    let total_post_vacuum_files = total_files_in_dir(&data_path);

    assert!(total_pre_vacuum_files > total_post_vacuum_files);
}
