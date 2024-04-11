mod fixtures;
mod utils;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;
use std::path::{Path, PathBuf};
use utils::*;
use walkdir::WalkDir;

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

    let total_pre_vacuum_files = total_parquet_files_in_dir(&data_path);

    "DROP TABLE t, s".execute(&mut conn);
    "VACUUM".execute(&mut conn);

    let total_post_vacuum_files = total_parquet_files_in_dir(&data_path);

    assert!(total_pre_vacuum_files > total_post_vacuum_files);
}

#[rstest]
fn vacuum_full_check_files(mut conn: PgConnection) {
    "CREATE TABLE t (a int) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1), (2), (3)".execute(&mut conn);
    "INSERT INTO t VALUES (4), (5), (6)".execute(&mut conn);

    let data_path = test_data_path(&mut conn);

    let total_pre_vacuum_files = total_parquet_files_in_dir(&data_path);

    "VACUUM FULL".execute(&mut conn);

    let total_post_vacuum_files = total_parquet_files_in_dir(&data_path);

    assert!(total_pre_vacuum_files > total_post_vacuum_files);
}
