mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn select_user_session_logs(mut conn: PgConnection) {
    UserSessionLogsTable::setup().execute(&mut conn);

    let columns: UserSessionLogsTableVec =
        "SELECT * FROM user_session_logs ORDER BY id".fetch_collect(&mut conn);

    // Check that the first ten ids are in order.
    let ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    assert_eq!(&columns.id[0..10], ids, "ids are in expected order");
    let event_names =
        "Login,Purchase,Logout,Signup,ViewProduct,AddToCart,RemoveFromCart,Checkout,Payment,Review";

    assert_eq!(
        &columns.event_name[0..10],
        event_names.split(',').collect::<Vec<_>>(),
        "event names are in expected order"
    );
}

#[rstest]
fn select_research_project_arrays(mut conn: PgConnection) {
    ResearchProjectArraysTable::setup().execute(&mut conn);

    let rows: Vec<ResearchProjectArraysTable> =
        "SELECT * FROM research_project_arrays".fetch_collect(&mut conn);

    // Using defaults for fields below that are unimplemented.
    let first = ResearchProjectArraysTable {
        project_id: Default::default(),
        experiment_flags: Some(vec![true, false, true]),
        binary_data: None,
        notes: Some(vec![
            "Initial setup complete".into(),
            "Preliminary results promising".into(),
        ]),
        keywords: Some(vec!["climate change".into(), "coral reefs".into()]),
        short_descriptions: Some(vec!["CRLRST    ".into(), "OCEAN1    ".into()]),
        participant_ages: Some(vec![28, 34, 29]),
        participant_ids: Some(vec![101, 102, 103]),
        observation_counts: Some(vec![150, 120, 130]),
        related_project_o_ids: None,
        measurement_errors: Some(vec![0.02, 0.03, 0.015]),
        precise_measurements: Some(vec![1.5, 1.6, 1.7]),
        observation_timestamps: None,
        observation_dates: None,
        budget_allocations: None,
        participant_uuids: None,
    };

    let second = ResearchProjectArraysTable {
        project_id: Default::default(),
        experiment_flags: Some(vec![false, true, false]),
        binary_data: None,
        notes: Some(vec![
            "Need to re-evaluate methodology".into(),
            "Unexpected results in phase 2".into(),
        ]),
        keywords: Some(vec!["sustainable farming".into(), "soil health".into()]),
        short_descriptions: Some(vec!["FARMEX    ".into(), "SOILQ2    ".into()]),
        participant_ages: Some(vec![22, 27, 32]),
        participant_ids: Some(vec![201, 202, 203]),
        observation_counts: Some(vec![160, 140, 135]),
        related_project_o_ids: None,
        measurement_errors: Some(vec![0.025, 0.02, 0.01]),
        precise_measurements: Some(vec![2.0, 2.1, 2.2]),
        observation_timestamps: None,
        observation_dates: None,
        budget_allocations: None,
        participant_uuids: None,
    };

    assert_eq!(rows[0], first);
    assert_eq!(rows[1], second);
}
