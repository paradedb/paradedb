mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn update_simple(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    match "UPDATE user_session_logs SET revenue = 100.0 WHERE user_id > 1".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: UPDATE is not supported because Parquet tables are append only."),
        _ => panic!("UPDATE should not be supported"),
    };
}

#[rstest]
async fn update_with_cte(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    match "WITH cte AS (SELECT * FROM user_session_logs WHERE user_id > 0) UPDATE user_session_logs SET revenue = 100.0 WHERE user_id in (SELECT user_id FROM cte)".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: failed to fetch tuple being updated"),
        _ => panic!("UPDATE should not be supported"),
    };
}
