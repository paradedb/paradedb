mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const TEST_TABLESPACE: &str = "test_tablespace";

fn custom_tablespace_path(conn: &mut PgConnection) -> PathBuf {
    let data_dir = "SHOW data_directory".fetch_one::<(String,)>(conn).0;
    let path = PathBuf::from(&data_dir).join(TEST_TABLESPACE);

    if path.exists() {
        std::fs::remove_dir_all(&path).unwrap();
    }

    std::fs::create_dir(&path).unwrap();
    path
}

fn total_files_in_dir(path: &Path) -> usize {
    WalkDir::new(path).into_iter().count()
}

#[rstest]
fn table_with_tablespace(mut conn: PgConnection) {
    let custom_tablespace_path = custom_tablespace_path(&mut conn);

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

    assert!(
        total_files_in_dir(
            &custom_tablespace_path
                .join("deltalake")
                .join(database_oid(&mut conn))
                .join(schema_oid(&mut conn, "my_schema"))
        ) > 0
    );
    assert!(
        total_files_in_dir(
            &custom_tablespace_path
                .join("deltalake")
                .join(database_oid(&mut conn))
                .join(schema_oid(&mut conn, "public"))
        ) > 0
    );
    assert!(total_files_in_dir(&default_schema_path(&mut conn, "my_schema")) > 0);
    assert!(total_files_in_dir(&default_schema_path(&mut conn, "public")) > 0);

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
