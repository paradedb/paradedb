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

fn total_files_in_dir(path: &Path) -> usize {
    WalkDir::new(path)
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
fn vacuum_full_table(mut conn: PgConnection) {
    "CREATE TABLE t (a int) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1), (2), (3)".execute(&mut conn);
    "INSERT INTO t VALUES (4), (5), (6)".execute(&mut conn);

    let data_path = default_table_path(&mut conn, "public", "t");
    let total_pre_vacuum_files = total_files_in_dir(&data_path);

    "VACUUM FULL t".execute(&mut conn);

    let total_post_vacuum_files = total_files_in_dir(&data_path);
    assert!(total_pre_vacuum_files > total_post_vacuum_files);
}

#[rstest]
fn vacuum_full_all(mut conn: PgConnection) {
    "CREATE TABLE t (a int) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1), (2), (3)".execute(&mut conn);
    "INSERT INTO t VALUES (4), (5), (6)".execute(&mut conn);

    "CREATE TABLE s (a int) USING parquet".execute(&mut conn);
    "INSERT INTO s VALUES (1), (2), (3)".execute(&mut conn);
    "INSERT INTO s VALUES (4), (5), (6)".execute(&mut conn);

    let total_pre_vacuum_files_t =
        total_files_in_dir(&default_table_path(&mut conn, "public", "t"));
    let total_pre_vacuum_files_s =
        total_files_in_dir(&default_table_path(&mut conn, "public", "s"));

    "VACUUM FULL".execute(&mut conn);

    let total_post_vacuum_files_t =
        total_files_in_dir(&default_table_path(&mut conn, "public", "t"));
    let total_post_vacuum_files_s =
        total_files_in_dir(&default_table_path(&mut conn, "public", "s"));

    assert!(total_pre_vacuum_files_t > total_post_vacuum_files_t);
    assert!(total_pre_vacuum_files_s > total_post_vacuum_files_s);
}
