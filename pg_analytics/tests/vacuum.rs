mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;
use std::path::Path;
use walkdir::WalkDir;

fn path_is_parquet_file(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => ext == "parquet",
        None => false,
    }
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

    let data_path = format!(
        "{}/{}",
        "SHOW data_directory".fetch_one::<(String,)>(&mut conn).0,
        "deltalake"
    );
    let total_pre_vacuum_files = WalkDir::new(data_path.clone())
        .contents_first(true)
        .into_iter()
        .filter(|e| path_is_parquet_file(e.as_ref().unwrap().path()))
        .count();

    "DROP TABLE t, s".execute(&mut conn);
    "VACUUM".execute(&mut conn);

    let total_post_vacuum_files = WalkDir::new(data_path.clone())
        .contents_first(true)
        .into_iter()
        .filter(|e| path_is_parquet_file(e.as_ref().unwrap().path()))
        .count();

    assert!(total_pre_vacuum_files > total_post_vacuum_files);
}

#[rstest]
fn vacuum_full_check_files(mut conn: PgConnection) {
    "CREATE TABLE t (a int) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1), (2), (3)".execute(&mut conn);
    "INSERT INTO t VALUES (4), (5), (6)".execute(&mut conn);

    let data_path = format!(
        "{}/{}",
        "SHOW data_directory".fetch_one::<(String,)>(&mut conn).0,
        "deltalake"
    );
    let total_pre_vacuum_files = WalkDir::new(data_path.clone())
        .into_iter()
        .filter(|e| path_is_parquet_file(e.as_ref().unwrap().path()))
        .count();

    "VACUUM FULL".execute(&mut conn);

    let total_post_vacuum_files = WalkDir::new(data_path.clone())
        .contents_first(true)
        .into_iter()
        .filter(|e| path_is_parquet_file(e.as_ref().unwrap().path()))
        .count();

    assert!(total_pre_vacuum_files > total_post_vacuum_files);
}
