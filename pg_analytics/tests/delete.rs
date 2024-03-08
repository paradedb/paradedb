mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn delete(mut conn: PgConnection) {
    UserSessionLogsTable::setup().execute(&mut conn);

    "DELETE FROM user_session_logs WHERE id >= 6".execute(&mut conn);

    let rows: UserSessionLogsRows = "SELECT * FROM user_session_logs".fetch(&mut conn);
    assert_eq!(rows.len(), 5);
}
