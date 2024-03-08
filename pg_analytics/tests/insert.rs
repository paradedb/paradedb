mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn insert_user_session_logs(mut conn: PgConnection) {
    UserSessionLogsTable::setup().execute(&mut conn);

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
    UserSessionLogsTable::setup().execute(&mut conn);

    r#"
    INSERT INTO user_session_logs
    (event_date, user_id, event_name, session_duration, page_views, revenue)
    VALUES
    (null, null, null, null, null, null);
    "#
    .execute(&mut conn);

    let rows: UserSessionLogsRows =
        "SELECT * FROM user_session_logs WHERE event_date IS NULL".fetch(&mut conn);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, Some(21));
    assert_eq!(rows[0].1, None);
    assert_eq!(rows[0].2, None);
    assert_eq!(rows[0].3, None);
    assert_eq!(rows[0].4, None);
    assert_eq!(rows[0].5, None);
    assert_eq!(rows[0].6, None);
}

#[rstest]
fn insert_research_project_arrays_with_null(mut conn: PgConnection) {
    ResearchProjectArraysTable::setup().execute(&mut conn);

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
