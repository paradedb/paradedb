mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;
use time::{macros::format_description, Time};

#[rstest]
fn insert_time(mut conn: PgConnection) {
    let time_format = format_description!("[hour]:[minute]:[second]");
    let time_micro_format = format_description!("[hour]:[minute]:[second].[subsecond]");

    r#"
        CREATE TABLE t (
            time_default TIME,
            time_micro TIME(6)
        ) USING parquet;
    "#
    .execute(&mut conn);

    r#"
        INSERT INTO t (time_default, time_micro)
        VALUES ('12:00:00', '12:00:00'),
               ('12:00:00.123', '12:00:00.123'),
               ('12:00:00.123456', '12:00:00.123456'),
               ('12:00:00.123457', '12:00:00.123457');
    "#
    .execute(&mut conn);

    let rows: Vec<(Time, Time)> = "SELECT * FROM t".fetch(&mut conn);
    let times = vec![
        Time::parse("12:00:00", time_format).unwrap(),
        Time::parse("12:00:00.123", time_micro_format).unwrap(),
        Time::parse("12:00:00.123456", time_micro_format).unwrap(),
        Time::parse("12:00:00.123457", time_micro_format).unwrap(),
    ];

    assert!(rows.iter().map(|r| r.0).eq(times.clone()));
    assert!(rows.iter().map(|r| r.1).eq(times.clone()));
}

#[rstest]
fn compare_times(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (
            time_default TIME,
            time_micro TIME(6)
        ) USING parquet;
    "#
    .execute(&mut conn);

    r#"
        INSERT INTO t (time_default, time_micro)
        VALUES ('12:00:00', '12:00:00'),
            ('12:00:00.123', '12:00:00.123'),
            ('12:00:00.123456', '12:00:00.123456'),
            ('12:00:00.123457', '12:00:00.123457');
    "#
    .execute(&mut conn);

    let rows: Vec<(Time, Time)> =
        "SELECT * FROM t WHERE time_default < '12:00:00.123457'".fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(Time, Time)> =
        "SELECT * FROM t WHERE time_default < '12:00:00.123456'".fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(Time, Time)> =
        "SELECT * FROM t WHERE time_micro < '12:00:00.123457'".fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(Time, Time)> =
        "SELECT * FROM t WHERE time_micro < '12:00:00.123456'".fetch(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn time_second(mut conn: PgConnection) {
    match "CREATE TABLE s (a time(0)) USING parquet".fetch_result::<()>(&mut conn) {
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: Only time and time(6), not time(0), are supported"
        ),
        _ => panic!("time(0) should not be supported"),
    }
}

#[rstest]
fn time_millisecond(mut conn: PgConnection) {
    match "CREATE TABLE s (a time(3)) USING parquet".fetch_result::<()>(&mut conn) {
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: Only time and time(6), not time(3), are supported"
        ),
        _ => panic!("time(0) should not be supported"),
    }
}

#[rstest]
fn time_not_supported(mut conn: PgConnection) {
    match "CREATE TABLE s (a time(2)) USING parquet".fetch_result::<()>(&mut conn) {
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: Only time and time(6), not time(2), are supported"
        ),
        _ => panic!("time(0) should not be supported"),
    }
}
