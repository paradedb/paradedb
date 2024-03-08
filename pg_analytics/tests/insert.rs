mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn insert_user_session_logs(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    r#"
        INSERT INTO user_session_logs
        (event_date, user_id, event_name, session_duration, page_views, revenue)
        VALUES
        ('2024-02-01', 2, 'Login', 200, 4, 25.00);
    "#
    .execute(&mut conn);

    let count: (i64,) =
        "SELECT COUNT(*) FROM user_session_logs WHERE event_date = '2024-02-01'::date"
            .fetch_one(&mut conn);
    assert_eq!(count, (1,));
}

#[rstest]
fn insert_user_session_logs_with_null(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    r#"
        INSERT INTO user_session_logs
        (event_date, user_id, event_name, session_duration, page_views, revenue)
        VALUES
        (null, null, null, null, null, null);
    "#
    .execute(&mut conn);

    let rows: Vec<UserSessionLogsTable> =
        "SELECT * FROM user_session_logs WHERE event_date IS NULL".fetch(&mut conn);

    let first = UserSessionLogsTable {
        id: 21,
        event_date: None,
        user_id: None,
        event_name: None,
        session_duration: None,
        page_views: None,
        revenue: None,
    };

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], first);
}

#[rstest]
fn insert_research_project_arrays_with_null(mut conn: PgConnection) {
    ResearchProjectArraysTable::setup_parquet().execute(&mut conn);

    r#"
        INSERT INTO research_project_arrays (experiment_flags) VALUES (NULL);
    "#
    .execute(&mut conn);

    let rows: Vec<ResearchProjectArraysTable> =
        "SELECT * FROM research_project_arrays WHERE experiment_flags IS NULL"
            .fetch_collect(&mut conn);

    let first = ResearchProjectArraysTable {
        project_id: Default::default(),
        experiment_flags: None,
        binary_data: None,
        notes: None,
        keywords: None,
        short_descriptions: None,
        participant_ages: None,
        participant_ids: None,
        observation_counts: None,
        related_project_o_ids: None,
        measurement_errors: None,
        precise_measurements: None,
        observation_timestamps: None,
        observation_dates: None,
        budget_allocations: None,
        participant_uuids: None,
    };

    assert_eq!(rows[0], first);
}

#[rstest]
fn insert_not_null(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text NOT NULL) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1, 'test');".execute(&mut conn);

    let row: (i32, String) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row, (1, "test".into()));

    match "INSERT INTO t VALUES (1)".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("should not be able to insert null into non-nullable column"),
        Err(err) => assert!(err.to_string().contains("error returned from database")),
    };
}

#[rstest]
fn insert_from_series(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (
            id INT
        ) USING parquet;
        INSERT INTO t (id) SELECT generate_series(1, 100000);
        INSERT INTO t (id) SELECT generate_series(1, 100000);
    "#
    .execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (200000,));

    r#"
        CREATE TABLE s (
            id INT
        ) USING parquet;
        INSERT INTO s (id) SELECT generate_series(1, 100000);
        DELETE FROM s WHERE id <= 50000;
        INSERT INTO s (id) SELECT generate_series(1, 100000);
    "#
    .execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM s".fetch_one(&mut conn);
    assert_eq!(count, (150000,));
}

#[rstest]
fn insert_parquet_from_parquet(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);
    r#"
        CREATE TABLE copy (
            id SERIAL PRIMARY KEY,
            event_date DATE,
            user_id INT,
            event_name VARCHAR(50),
            session_duration INT,
            page_views INT,
            revenue DECIMAL(10, 2)
        ) USING parquet;
        INSERT INTO copy SELECT * FROM user_session_logs;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT id, event_name FROM copy".fetch(&mut conn);

    let ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let event_names =
        "Login,Purchase,Logout,Signup,ViewProduct,AddToCart,RemoveFromCart,Checkout,Payment,Review"
            .split(',');

    assert!(rows.iter().take(10).map(|r| r.0).eq(ids));
    assert!(rows.iter().take(10).map(|r| r.1.clone()).eq(event_names));
}

#[rstest]
fn insert_parquet_from_heap(mut conn: PgConnection) {
    UserSessionLogsTable::setup_heap().execute(&mut conn);
    r#"
        CREATE TABLE copy (
            id SERIAL PRIMARY KEY,
            event_date DATE,
            user_id INT,
            event_name VARCHAR(50),
            session_duration INT,
            page_views INT,
            revenue DECIMAL(10, 2)
        ) USING parquet;
        INSERT INTO copy SELECT * FROM user_session_logs;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT id, event_name FROM copy".fetch(&mut conn);

    let ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let event_names =
        "Login,Purchase,Logout,Signup,ViewProduct,AddToCart,RemoveFromCart,Checkout,Payment,Review"
            .split(',');

    assert!(rows.iter().take(10).map(|r| r.0).eq(ids));
    assert!(rows.iter().take(10).map(|r| r.1.clone()).eq(event_names));
}
