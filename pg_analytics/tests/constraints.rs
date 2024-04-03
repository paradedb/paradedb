mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn primary_key(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (id INT PRIMARY KEY, name TEXT) USING parquet;
    "#
    .execute(&mut conn);

    match "INSERT INTO t VALUES (1, 'test'), (1, 'test');".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains(
            "error returned from database: duplicate key value violates unique constraint"
        )),
        _ => panic!("Primary key constraint violated"),
    };

    match "INSERT INTO t VALUES (2, 'test'); INSERT INTO t VALUES (2, 'test');"
        .execute_result(&mut conn)
    {
        Err(err) => assert!(err.to_string().contains(
            "error returned from database: duplicate key value violates unique constraint"
        )),
        _ => panic!("Primary key constraint violated"),
    };

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (0,));
}

#[rstest]
async fn unique(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (id INT UNIQUE, name TEXT) USING parquet;
    "#
    .execute(&mut conn);

    match "INSERT INTO t VALUES (1, 'test'), (1, 'test');".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains(
            "error returned from database: duplicate key value violates unique constraint"
        )),
        _ => panic!("Primary key constraint violated"),
    };

    match "INSERT INTO t VALUES (2, 'test'); INSERT INTO t VALUES (2, 'test');"
        .execute_result(&mut conn)
    {
        Err(err) => assert!(err.to_string().contains(
            "error returned from database: duplicate key value violates unique constraint"
        )),
        _ => panic!("Primary key constraint violated"),
    };

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (0,));
}

#[rstest]
async fn check(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (id INT PRIMARY KEY CHECK (id > 0), name TEXT) USING parquet;
    "#
    .execute(&mut conn);

    match "INSERT INTO t VALUES (1, 'test'), (1, 'test');".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains(
            "error returned from database: duplicate key value violates unique constraint"
        )),
        _ => panic!("Primary key constraint violated"),
    };

    match "INSERT INTO t VALUES (-1, 'test'); INSERT INTO t VALUES (2, 'test');"
        .execute_result(&mut conn)
    {
        Err(err) => assert!(err.to_string().contains("violates check constraint")),
        _ => panic!("Check constraint violated"),
    };

    "INSERT INTO t VALUES (1, 'test')".execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (1,));
}

#[rstest]
async fn default(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (id INT DEFAULT 4, name TEXT) USING parquet;
    "#
    .execute(&mut conn);

    "INSERT INTO t (name) VALUES ('abc'), ('efg')".execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT id, name FROM t".fetch(&mut conn);

    let ids = vec![4, 4];
    let names = vec!["abc", "efg"];

    assert!(rows.iter().map(|r| r.0).eq(ids));
    assert!(rows.iter().map(|r| r.1.clone()).eq(names));
}
