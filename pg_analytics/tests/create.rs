mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn create_heap_from_parquet(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);
    "CREATE TABLE copy AS SELECT * FROM user_session_logs".execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT id, event_name FROM user_session_logs".fetch(&mut conn);

    let ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let event_names =
        "Login,Purchase,Logout,Signup,ViewProduct,AddToCart,RemoveFromCart,Checkout,Payment,Review"
            .split(',');

    assert!(rows.iter().take(10).map(|r| r.0).eq(ids));
    assert!(rows.iter().take(10).map(|r| r.1.clone()).eq(event_names));
}

#[rstest]
fn create_heap_from_parquet_with_select(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);
    "SELECT * INTO copy from user_session_logs".execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT id, event_name FROM user_session_logs".fetch(&mut conn);

    let ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let event_names =
        "Login,Purchase,Logout,Signup,ViewProduct,AddToCart,RemoveFromCart,Checkout,Payment,Review"
            .split(',');

    assert!(rows.iter().take(10).map(|r| r.0).eq(ids));
    assert!(rows.iter().take(10).map(|r| r.1.clone()).eq(event_names));
}
