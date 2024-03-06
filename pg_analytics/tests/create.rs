mod fixtures;

use fixtures::*;
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

#[rstest]
fn create_parquet_from_heap(mut conn: PgConnection) {
    r#"
        CREATE TABLE t_heap (
            id INT,
            name TEXT
        );
        INSERT INTO t_heap VALUES (1, 'abc'), (2, 'def'), (3, 'hij');
        CREATE TABLE t_heap_parquet_copy USING parquet AS SELECT * FROM t_heap;
    "#
    .execute(&mut conn);

    let mut rows: Vec<(i32,)> = "SELECT id FROM t_heap_parquet_copy".fetch(&mut conn);
    let mut ids: Vec<i32> = rows.into_iter().map(|r| r.0).collect();
    ids.sort();
    assert_eq!(ids, [1, 2, 3]);
}
