mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;
use time::{macros::format_description, PrimitiveDateTime};

#[rstest]
fn insert_timestamp(mut conn: PgConnection) {
    let timestamp_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

    r#"
        CREATE TABLE t (
            timestamp_default TIMESTAMP,
            timestamp_micro TIMESTAMP(6)
        ) USING parquet;
    "#
    .execute(&mut conn);

    r#"
        INSERT INTO t (timestamp_default, timestamp_micro)
        VALUES ('0001-02-02 12:00:00.123', '0001-02-02 12:00:00.123'),
               ('1230-02-03 12:00:00', '1230-02-03 12:00:00'),
               ('2000-02-04 12:00:00.123456', '2000-02-04 12:00:00.123456'),
               ('2000-02-04 12:00:00.123457', '2000-02-04 12:00:00.123457');
    "#
    .execute(&mut conn);

    let rows: Vec<(PrimitiveDateTime, PrimitiveDateTime)> = "SELECT * FROM t".fetch(&mut conn);
    let timestamps = vec![
        PrimitiveDateTime::parse("0001-02-02 12:00:00", timestamp_format).unwrap(),
        PrimitiveDateTime::parse("1230-02-03 12:00:00", timestamp_format).unwrap(),
        PrimitiveDateTime::parse("2000-02-04 12:00:00", timestamp_format).unwrap(),
        PrimitiveDateTime::parse("2000-02-04 12:00:00", timestamp_format).unwrap(),
    ];

    assert!(rows.iter().map(|r| r.0).eq(timestamps.clone()));
    assert!(rows.iter().map(|r| r.1).eq(timestamps.clone()));
}

#[rstest]
fn compare_timestamps(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (
            timestamp_default TIMESTAMP,
            timestamp_micro TIMESTAMP(6)
        ) USING parquet;
    "#
    .execute(&mut conn);

    r#"
        INSERT INTO t (timestamp_default, timestamp_micro)
        VALUES ('0001-02-02 12:00:00.123', '0001-02-02 12:00:00.123'),
            ('1230-02-03 12:00:00', '1230-02-03 12:00:00'),
            ('2000-02-04 12:00:00.123456', '2000-02-04 12:00:00.123456'),
            ('2000-02-04 12:00:00.123457', '2000-02-04 12:00:00.123457');
    "#
    .execute(&mut conn);

    let rows: Vec<(PrimitiveDateTime, PrimitiveDateTime)> =
        "SELECT * FROM t WHERE timestamp_default < '2000-02-04 12:00:00.123457'::timestamp"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(PrimitiveDateTime, PrimitiveDateTime)> =
        "SELECT * FROM t WHERE timestamp_default < '2000-02-04 12:00:00.123456'::timestamp"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(PrimitiveDateTime, PrimitiveDateTime)> =
        "SELECT * FROM t WHERE timestamp_micro < '2000-02-04 12:00:00.123457'::timestamp"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(PrimitiveDateTime, PrimitiveDateTime)> =
        "SELECT * FROM t WHERE timestamp_micro < '2000-02-04 12:00:00.123456'::timestamp"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn timestamp_second(mut conn: PgConnection) {
    match "CREATE TABLE s (a timestamp(0)) USING parquet".fetch_result::<()>(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: Only timestamp and timestamp(6), not timestamp(0), are supported"),
        _ => panic!("timestamp(0) should not be supported"),
    }
}

#[rstest]
fn timestamp_millisecond(mut conn: PgConnection) {
    match "CREATE TABLE s (a timestamp(3)) USING parquet".fetch_result::<()>(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: Only timestamp and timestamp(6), not timestamp(3), are supported"),
        _ => panic!("timestamp(0) should not be supported"),
    }
}

#[rstest]
fn timestamp_not_supported(mut conn: PgConnection) {
    match "CREATE TABLE s (a timestamp(2)) USING parquet".fetch_result::<()>(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: Only timestamp and timestamp(6), not timestamp(2), are supported"),
        _ => panic!("timestamp(0) should not be supported"),
    }
}
