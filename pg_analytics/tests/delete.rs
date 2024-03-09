mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn delete(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    "DELETE FROM user_session_logs WHERE id >= 6".execute(&mut conn);

    let rows: Vec<UserSessionLogsTable> = "SELECT * FROM user_session_logs".fetch(&mut conn);
    assert_eq!(rows.len(), 5);
}

#[rstest]
fn truncate(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    "TRUNCATE user_session_logs;".execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT * FROM user_session_logs".fetch(&mut conn);
    assert!(rows.is_empty())
}
