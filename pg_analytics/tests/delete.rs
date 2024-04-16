mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn delete_simple(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    match "DELETE FROM user_session_logs WHERE id >= 6".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: DELETE is not currently supported because Parquet tables are append only."),
        _ => panic!("DELETE should not be supported"),
    };
}

#[rstest]
fn delete_with_cte(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    match "WITH d AS (DELETE FROM user_session_logs WHERE id > 0 RETURNING *) SELECT COUNT(*) FROM d".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: DELETE is not currently supported because Parquet tables are append only."),
        _ => panic!("DELETE should not be supported"),
    };
}

#[rstest]
fn truncate(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    "TRUNCATE user_session_logs;".execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT * FROM user_session_logs".fetch(&mut conn);
    assert!(rows.is_empty())
}

#[rstest]
fn federated_delete(mut conn: PgConnection) {
    "CREATE TABLE u ( name TEXT, age INTEGER ) USING parquet".execute(&mut conn);
    "CREATE TABLE v ( name TEXT )".execute(&mut conn);
    r#"
    INSERT INTO u (name, age) VALUES
    ('Alice', 101),
    ('Bob', 102),
    ('Charlie', 103),
    ('David', 101);
    INSERT INTO v (name) VALUES
    ('Alice'),
    ('Bob');
    "#
    .execute(&mut conn);

    match "DELETE FROM u WHERE name IN (SELECT name FROM v)".execute_result(&mut conn) {
        Err(err) => assert!(err
            .to_string()
            .contains("DELETE is not currently supported")),
        _ => panic!("Federated DML should not be supported"),
    };
}
