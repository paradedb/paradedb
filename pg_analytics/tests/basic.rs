mod fixtures;

use fixtures::*;
use rstest::*;

#[rstest]
#[ignore]
fn basic_select(mut user_session_log_table: TableConnection<UserSessionLogsTable>) {
    let columns: UserSessionLogsTableVec =
        user_session_log_table.fetch_collect("SELECT * FROM user_session_logs ORDER BY id");

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
#[ignore]
fn array_results(mut research_project_arrays_table: TableConnection<ResearchProjectArraysTable>) {
    let columns: Vec<ResearchProjectArraysTable> =
        research_project_arrays_table.fetch_collect("SELECT * FROM research_project_arrays");

    // Using defaults for fields below that are unimplemented.
    let first = ResearchProjectArraysTable {
        project_id: Default::default(),
        experiment_flags: vec![false, true, false],
        binary_data: Default::default(),
        notes: vec![
            "Need to re-evaluate methodology".into(),
            "Unexpected results in phase 2".into(),
        ],
        keywords: vec!["sustainable farming".into(), "soil health".into()],
        short_descriptions: vec!["FARMEX    ".into(), "SOILQ2    ".into()],
        participant_ages: vec![22, 27, 32],
        participant_ids: vec![201, 202, 203],
        observation_counts: vec![160, 140, 135],
        related_project_o_ids: Default::default(),
        measurement_errors: vec![0.025, 0.02, 0.01],
        precise_measurements: vec![2.0, 2.1, 2.2],
        observation_timestamps: Default::default(),
        observation_dates: Default::default(),
        budget_allocations: Default::default(),
        participant_uuids: Default::default(),
    };

    let second = ResearchProjectArraysTable {
        project_id: Default::default(),
        experiment_flags: vec![true, false, true],
        binary_data: Default::default(),
        notes: vec![
            "Initial setup complete".into(),
            "Preliminary results promising".into(),
        ],
        keywords: vec!["climate change".into(), "coral reefs".into()],
        short_descriptions: vec!["CRLRST    ".into(), "OCEAN1    ".into()],
        participant_ages: vec![28, 34, 29],
        participant_ids: vec![101, 102, 103],
        observation_counts: vec![150, 120, 130],
        related_project_o_ids: Default::default(),
        measurement_errors: vec![0.02, 0.03, 0.015],
        precise_measurements: vec![1.5, 1.6, 1.7],
        observation_timestamps: Default::default(),
        observation_dates: Default::default(),
        budget_allocations: Default::default(),
        participant_uuids: Default::default(),
    };

    assert_eq!(columns[0], first);
    assert_eq!(columns[1], second);
}
