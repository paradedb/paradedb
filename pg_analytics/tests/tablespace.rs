mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const TEST_TABLESPACE: &str = "test_tablespace";
const DELTALAKE_DIR: &str = "deltalake";

fn custom_tablespace_path(conn: &mut PgConnection) -> PathBuf {
    let data_dir = "SHOW data_directory".fetch_one::<(String,)>(conn).0;
    let path = PathBuf::from(&data_dir).join(TEST_TABLESPACE);

    if path.exists() {
        std::fs::remove_dir_all(&path).unwrap();
    }

    std::fs::create_dir(&path).unwrap();
    path
}

fn default_tablespace_path(conn: &mut PgConnection) -> PathBuf {
    let data_dir = "SHOW data_directory".fetch_one::<(String,)>(conn).0;
    PathBuf::from(&data_dir).join(DELTALAKE_DIR)
}

fn total_files_in_dir(path: &Path) -> usize {
    WalkDir::new(path).into_iter().count()
}

#[rstest]
fn table_with_tablespace(mut conn: PgConnection) {
    let custom_tablespace_path = custom_tablespace_path(&mut conn);
    let default_tablespace_path = default_tablespace_path(&mut conn);

    format!(
        r#"
            CREATE TABLESPACE {} LOCATION '{}';
        "#,
        TEST_TABLESPACE,
        custom_tablespace_path.display()
    )
    .execute(&mut conn);

    format!(
        r#"
            CREATE SCHEMA my_schema;
            CREATE TABLE t (a int) USING parquet;
            CREATE TABLE s (a int) USING parquet TABLESPACE {TEST_TABLESPACE};
            CREATE TABLE my_schema.t (a int) USING parquet;
            CREATE TABLE my_schema.s (a int) USING parquet TABLESPACE {TEST_TABLESPACE};
            INSERT INTO t VALUES (1);
            INSERT INTO s VALUES (2);
            INSERT INTO my_schema.t VALUES (3);
            INSERT INTO my_schema.s VALUES (4);
        "#
    )
    .execute(&mut conn);

    let db_name = "SELECT current_database()"
        .fetch_one::<(String,)>(&mut conn)
        .0;
    let db_oid = format!("SELECT oid FROM pg_database WHERE datname='{db_name}'")
        .fetch_one::<(sqlx::postgres::types::Oid,)>(&mut conn)
        .0
         .0;
    let schema_oid = "SELECT oid FROM pg_namespace WHERE nspname='my_schema'"
        .to_string()
        .fetch_one::<(sqlx::postgres::types::Oid,)>(&mut conn)
        .0
         .0;

    assert!(custom_tablespace_path.join(DELTALAKE_DIR).exists());
    assert!(
        total_files_in_dir(
            &custom_tablespace_path
                .join(DELTALAKE_DIR)
                .join(db_oid.to_string())
                .join(schema_oid.to_string())
        ) > 0
    );
    assert!(
        total_files_in_dir(
            &default_tablespace_path
                .join(db_oid.to_string())
                .join(schema_oid.to_string())
        ) > 0
    );

    assert_eq!("SELECT a FROM t".fetch_one::<(i32,)>(&mut conn), (1,));
    assert_eq!("SELECT a FROM s".fetch_one::<(i32,)>(&mut conn), (2,));
    assert_eq!(
        "SELECT a FROM my_schema.t".fetch_one::<(i32,)>(&mut conn),
        (3,)
    );
    assert_eq!(
        "SELECT a FROM my_schema.s".fetch_one::<(i32,)>(&mut conn),
        (4,)
    );

    "DROP TABLE s, my_schema.s".execute(&mut conn);
    format!("DROP TABLESPACE {TEST_TABLESPACE}").execute(&mut conn);
}
