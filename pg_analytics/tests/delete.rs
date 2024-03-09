mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn delete_simple(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    match "DELETE FROM user_session_logs WHERE id >= 6".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: DELETE is not supported because Parquet tables are append only."),
        _ => panic!("DELETE should not be supported"),
    };
}

#[rstest]
fn delete_with_cte(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    match "WITH d AS (DELETE FROM user_session_logs WHERE id > 0 RETURNING *) SELECT COUNT(*) FROM d".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: DELETE is not supported because Parquet tables are append only."),
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
