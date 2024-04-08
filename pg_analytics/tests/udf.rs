mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn udf(mut conn: PgConnection) {
    r#"
        CREATE TABLE x (
            a INTEGER,
            b INTEGER
        ) USING parquet;
        INSERT INTO x VALUES (1, 2), (3, 4), (5, 6), (7, 8);
        CREATE FUNCTION add(integer, integer) RETURNS integer
            AS 'select $1 + $2;'
            LANGUAGE SQL
            IMMUTABLE
            RETURNS NULL ON NULL INPUT;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = "SELECT add(a, b) FROM x".fetch(&mut conn);
    let sums: Vec<i32> = rows.into_iter().map(|r| r.0).collect();
    assert_eq!(sums, [3, 7, 11, 15]);
}

#[rstest]
fn udf_overloaded(mut conn: PgConnection) {
    r#"
        CREATE TABLE x (
            a INTEGER,
            b INTEGER
        ) USING parquet;
        INSERT INTO x VALUES (1, 2), (3, 4), (5, 6), (7, 8);
        CREATE FUNCTION add(integer, integer) RETURNS integer
            AS 'select $1 + $2;'
            LANGUAGE SQL
            IMMUTABLE
            RETURNS NULL ON NULL INPUT;
        CREATE FUNCTION add(float8, float8) RETURNS float8
            AS 'select $1 + $2;'
            LANGUAGE SQL
            IMMUTABLE
            RETURNS NULL ON NULL INPUT;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = "SELECT add(a, b) FROM x".fetch(&mut conn);
    let sums: Vec<i32> = rows.into_iter().map(|r| r.0).collect();
    assert_eq!(sums, [3, 7, 11, 15]);
}

#[rstest]
fn udf_deletion(mut conn: PgConnection) {
    r#"
        CREATE TABLE x (
            a INTEGER,
            b INTEGER
        ) USING parquet;
        INSERT INTO x VALUES (1, 2), (3, 4), (5, 6), (7, 8);
        CREATE FUNCTION add(integer, integer) RETURNS integer
            AS 'select $1 + $2;'
            LANGUAGE SQL
            IMMUTABLE
            RETURNS NULL ON NULL INPUT;
        SELECT add(a, b) FROM x;
        DROP FUNCTION add;
    "#
    .execute(&mut conn);

    // This is the current behavior, but we want deletion to actually work in the future!
    match "SELECT add(a, b) FROM x".execute_result(&mut conn) {
        Err(err) => {
            assert!(err
                .to_string()
                .contains("function add(integer, integer) does not exist"))
        }
        _ => panic!("Deleted functions should not execute"),
    };
}

#[rstest]
fn udf_coercion(mut conn: PgConnection) {
    r#"
        CREATE TABLE x (
            a INTEGER,
            b INTEGER
        ) USING parquet;
        INSERT INTO x VALUES (1, 2), (3, 4), (5, 6), (7, 8);
        CREATE FUNCTION add(integer, integer) RETURNS integer
            AS 'select $1 + $2;'
            LANGUAGE SQL
            IMMUTABLE
            RETURNS NULL ON NULL INPUT;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = "SELECT add(1, b) FROM x".fetch(&mut conn);
    let sums: Vec<i32> = rows.into_iter().map(|r| r.0).collect();
    assert_eq!(sums, [3, 5, 7, 9]);
}
