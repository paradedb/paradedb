mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn rename(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    "ALTER TABLE user_session_logs RENAME TO t".execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT id, event_name FROM t".fetch(&mut conn);

    let ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let event_names =
        "Login,Purchase,Logout,Signup,ViewProduct,AddToCart,RemoveFromCart,Checkout,Payment,Review"
            .split(',');

    assert!(rows.iter().take(10).map(|r| r.0).eq(ids));
    assert!(rows.iter().take(10).map(|r| r.1.clone()).eq(event_names));
}

#[rstest]
fn add_column(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    match "ALTER TABLE user_session_logs ADD COLUMN a int".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: ADD COLUMN is not yet supported. Please recreate the table instead."),
        _ => panic!("Adding a column should not be supported"),
    };
}

#[rstest]
fn drop_column(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    match "ALTER TABLE user_session_logs DROP COLUMN user_id".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: DROP COLUMN is not yet supported. Please recreate the table instead."),
        _ => panic!("Dropping a column should not be supported"),
    };
}

#[rstest]
fn rename_column(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    match "ALTER TABLE user_session_logs RENAME COLUMN user_id TO a".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: RENAME COLUMN is not yet supported. Please recreate the table instead."),
        _ => panic!("Renaming a column should not be supported"),
    };
}

#[rstest]
#[ignore]
fn alter(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);

    let rows: Vec<(String,)> = "SELECT column_name FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 't'".fetch(&mut conn);
    let column_names: Vec<_> = rows.into_iter().map(|r| r.0).collect();

    assert_eq!(column_names, vec!["a".to_string(), "b".to_string()]);

    "ALTER TABLE t ADD COLUMN c int".execute(&mut conn);

    let rows: Vec<(String,)> = "SELECT column_name FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 't'".fetch(&mut conn);
    let column_names: Vec<_> = rows.into_iter().map(|r| r.0).collect();

    assert_eq!(
        column_names,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}
