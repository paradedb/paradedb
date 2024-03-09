mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn multiline_create_insert(mut conn: PgConnection) {
    "CREATE TABLE employees (salary bigint, id smallint) USING parquet; INSERT INTO employees VALUES (100, 1), (200, 2), (300, 3), (400, 4), (500, 5);".execute(&mut conn);
    let insert_count: (i64,) = "SELECT COUNT(*) FROM employees".fetch_one(&mut conn);
    assert_eq!(insert_count, (5,));
}

#[rstest]
fn multiline_create_insert_truncate(mut conn: PgConnection) {
    "CREATE TABLE t (id smallint) USING parquet; INSERT INTO t VALUES (1); TRUNCATE TABLE t;"
        .execute(&mut conn);
    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (0,));

    let rows: Vec<(String,)> =
        "SELECT column_name FROM information_schema.columns WHERE table_name = 't'"
            .fetch(&mut conn);
    let mut column_names: Vec<_> = rows.into_iter().map(|r| r.0).collect();
    assert_eq!(
        column_names.sort(),
        ["id".to_string(), "name".to_string()].sort()
    );
}
