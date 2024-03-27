mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;
use walkdir::WalkDir;

#[rstest]
fn vacuum(mut conn: PgConnection) {
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
        .into_iter()
        .filter_entry(|e| !e.path().to_str().unwrap().contains("_delta_log"))
        .count();

    "VACUUM".execute(&mut conn);
    "VACUUM FULL".execute(&mut conn);
    "VACUUM t".execute(&mut conn);
    "VACUUM FULL t".execute(&mut conn);
    "DROP TABLE t, s".execute(&mut conn);
    "VACUUM".execute(&mut conn);

    let total_post_vacuum_files = WalkDir::new(data_path.clone())
        .into_iter()
        .filter_entry(|e| !e.path().to_str().unwrap().contains("_delta_log"))
        .count();

    assert!(total_pre_vacuum_files > total_post_vacuum_files);
}
