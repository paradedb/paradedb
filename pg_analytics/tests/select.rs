mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::{types::BigDecimal, PgConnection};
use std::str::FromStr;
use time::{macros::format_description, Date, PrimitiveDateTime};

#[rstest]
fn select_user_session_logs(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT id, event_name FROM user_session_logs".fetch(&mut conn);

    let ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let event_names =
        "Login,Purchase,Logout,Signup,ViewProduct,AddToCart,RemoveFromCart,Checkout,Payment,Review"
            .split(',');

    assert!(rows.iter().take(10).map(|r| r.0).eq(ids));
    assert!(rows.iter().take(10).map(|r| r.1.clone()).eq(event_names));

    let rows: Vec<(Date, BigDecimal)> = r#"
    SELECT event_date, SUM(revenue) AS total_revenue
    FROM user_session_logs
    GROUP BY event_date
    ORDER BY event_date"#
        .fetch(&mut conn);

    let expected_dates = "
        2024-01-01,2024-01-02,2024-01-03,2024-01-04,2024-01-05,2024-01-06,2024-01-07,
        2024-01-08,2024-01-09,2024-01-10,2024-01-11,2024-01-12,2024-01-13,2024-01-14,
        2024-01-15,2024-01-16,2024-01-17,2024-01-18,2024-01-19,2024-01-20"
        .split(',')
        .map(|s| Date::parse(s.trim(), format_description!("[year]-[month]-[day]")).unwrap());

    let expected_revenues = "
        20.00,150.50,0.00,0.00,30.75,75.00,0.00,200.25,300.00,50.00,0.00,125.30,0.00,
        0.00,45.00,80.00,0.00,175.50,250.00,60.00"
        .split(',')
        .map(|s| BigDecimal::from_str(s.trim()).unwrap());

    assert!(rows.iter().map(|r| r.0).eq(expected_dates));
    assert!(rows.iter().map(|r| r.1.clone()).eq(expected_revenues));
}

#[rstest]
fn select_research_project_arrays(mut conn: PgConnection) {
    ResearchProjectArraysTable::setup_parquet().execute(&mut conn);

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

#[rstest]
fn select_across_schemas(mut conn: PgConnection) {
    r#"
        CREATE SCHEMA s1; 
        CREATE SCHEMA s2; 
        CREATE TABLE t (a int) USING parquet; 
        CREATE TABLE s1.u (a int) USING parquet; 
        CREATE TABLE s2.v (a int) USING parquet; 
        CREATE TABLE s1.t (a int) USING parquet; 
        CREATE TABLE s2.t (a int) USING parquet; 
        INSERT INTO t VALUES (0); 
        INSERT INTO s1.u VALUES (1); 
        INSERT INTO s2.v VALUES (2); 
        INSERT INTO s1.t VALUES (3); 
        INSERT INTO s2.t VALUES (4);
    "#
    .execute(&mut conn);

    assert_eq!("SELECT a FROM t".fetch_one::<(i32,)>(&mut conn), (0,));
    assert_eq!("SELECT a FROM s1.u".fetch_one::<(i32,)>(&mut conn), (1,));
    assert_eq!("SELECT a FROM s2.v".fetch_one::<(i32,)>(&mut conn), (2,));

    match "SELECT a FROM u".execute_result(&mut conn) {
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: relation \"u\" does not exist"
        ),
        _ => panic!("Was able to select schema not in search path"),
    };

    let _ = "SET search_path = public, s1, s2".execute_result(&mut conn);
    assert_eq!("SELECT a FROM t".fetch_one::<(i32,)>(&mut conn), (0,));
    assert_eq!("SELECT a FROM u".fetch_one::<(i32,)>(&mut conn), (1,));
    assert_eq!("SELECT a FROM v".fetch_one::<(i32,)>(&mut conn), (2,));

    let _ = "SET search_path = s2, s1, public".execute_result(&mut conn);
    assert_eq!("SELECT a FROM t".fetch_one::<(i32,)>(&mut conn), (4,));
    assert_eq!("SELECT a FROM u".fetch_one::<(i32,)>(&mut conn), (1,));

    let _ = "SET search_path = s1".execute_result(&mut conn);
    assert_eq!("SELECT a FROM t".fetch_one::<(i32,)>(&mut conn), (3,));
}
