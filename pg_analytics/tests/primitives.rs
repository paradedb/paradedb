mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::{types::BigDecimal, types::Uuid, PgConnection};
use std::str::FromStr;
use time::{macros::format_description, Date, PrimitiveDateTime, Time};

#[rstest]
fn text_type(mut conn: PgConnection) {
    "CREATE TABLE t (a text) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES ('hello world')".execute(&mut conn);
    let row: (String,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, "hello world".to_string());
}

#[rstest]
fn text_array_type(mut conn: PgConnection) {
    "CREATE TABLE t (a text[]) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (ARRAY['hello', 'world'])".execute(&mut conn);
    let row: (Vec<String>,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, vec!["hello", "world"]);
}

#[rstest]
fn varchar_type(mut conn: PgConnection) {
    "CREATE TABLE t (a varchar) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES ('hello world')".execute(&mut conn);
    let row: (String,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, "hello world".to_string());
}

#[rstest]
fn varchar_array_type(mut conn: PgConnection) {
    "CREATE TABLE t (a varchar[]) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (ARRAY['hello', 'world'])".execute(&mut conn);
    let row: (Vec<String>,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, vec!["hello", "world"]);
}

#[rstest]
fn char_type(mut conn: PgConnection) {
    "CREATE TABLE t (a char) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES ('h')".execute(&mut conn);
    let row: (String,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, "h".to_string());
}

#[rstest]
fn char_array_type(mut conn: PgConnection) {
    "CREATE TABLE t (a char[]) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (ARRAY['h', 'i'])".execute(&mut conn);
    let row: (Vec<String>,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, vec!["h", "i"]);
}

#[rstest]
fn smallint_type(mut conn: PgConnection) {
    "CREATE TABLE t (a smallint) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1)".execute(&mut conn);
    let row: (i16,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, 1);
}

#[rstest]
fn smallint_array_type(mut conn: PgConnection) {
    "CREATE TABLE t (a smallint[]) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (ARRAY[1, 2])".execute(&mut conn);
    let row: (Vec<i16>,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, vec![1, 2]);
}

#[rstest]
fn integer_type(mut conn: PgConnection) {
    "CREATE TABLE t (a integer) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1)".execute(&mut conn);
    let row: (i32,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, 1);
}

#[rstest]
fn integer_array_type(mut conn: PgConnection) {
    "CREATE TABLE t (a integer[]) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (ARRAY[1, 2])".execute(&mut conn);
    let row: (Vec<i32>,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, vec![1, 2]);
}

#[rstest]
fn bigint_type(mut conn: PgConnection) {
    "CREATE TABLE t (a bigint) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1)".execute(&mut conn);
    let row: (i64,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, 1);
}

#[rstest]
fn bigint_array_type(mut conn: PgConnection) {
    "CREATE TABLE t (a bigint[]) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (ARRAY[1, 2])".execute(&mut conn);
    let row: (Vec<i64>,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, vec![1, 2]);
}

#[rstest]
fn real_type(mut conn: PgConnection) {
    "CREATE TABLE t (a real) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1.0)".execute(&mut conn);
    let row: (f32,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, 1.0);
}

#[rstest]
fn real_array_type(mut conn: PgConnection) {
    "CREATE TABLE t (a real[]) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (ARRAY[1.0, 2.0])".execute(&mut conn);
    let row: (Vec<f32>,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, vec![1.0, 2.0]);
}

#[rstest]
fn double_type(mut conn: PgConnection) {
    "CREATE TABLE t (a double precision) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1.0)".execute(&mut conn);
    let row: (f64,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, 1.0);
}

#[rstest]
fn double_array_type(mut conn: PgConnection) {
    "CREATE TABLE t (a double precision[]) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (ARRAY[1.0, 2.0])".execute(&mut conn);
    let row: (Vec<f64>,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, vec![1.0, 2.0]);
}

#[rstest]
fn bool_type(mut conn: PgConnection) {
    "CREATE TABLE t (a bool) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (true)".execute(&mut conn);
    let row: (bool,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, true);
}

#[rstest]
fn bool_array_type(mut conn: PgConnection) {
    "CREATE TABLE t (a bool[]) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (ARRAY[true, false])".execute(&mut conn);
    let row: (Vec<bool>,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, vec![true, false]);
}

#[rstest]
fn numeric_type(mut conn: PgConnection) {
    "CREATE TABLE t (a numeric(5, 2)) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1.01)".execute(&mut conn);
    let row: (BigDecimal,) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row.0, BigDecimal::from_str("1.01").unwrap());
}

#[rstest]
fn time_type(mut conn: PgConnection) {
    "CREATE TABLE t (a time) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES ('15:30:00')".execute(&mut conn);
    let row: (Time,) = "SELECT * FROM t".fetch_one(&mut conn);
    let fd = format_description!("[hour]:[minute]:[second]");
    assert_eq!(row.0, Time::parse("15:30:00", fd).unwrap());
}

#[rstest]
fn timestamp_type(mut conn: PgConnection) {
    "CREATE TABLE t (a timestamp) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES ('2024-01-29 15:30:00')".execute(&mut conn);
    let row: (PrimitiveDateTime,) = "SELECT * FROM t".fetch_one(&mut conn);
    let fd = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    assert_eq!(
        row.0,
        PrimitiveDateTime::parse("2024-01-29 15:30:00", fd).unwrap()
    );
}

#[rstest]
fn date_type(mut conn: PgConnection) {
    "CREATE TABLE t (a date) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES ('2024-01-29')".execute(&mut conn);
    let row: (Date,) = "SELECT * FROM t".fetch_one(&mut conn);
    let fd = format_description!("[year]-[month]-[day]");
    assert_eq!(row.0, Date::parse("2024-01-29", fd).unwrap());
}

#[rstest]
fn byte_type(mut conn: PgConnection) {
    match "CREATE TABLE t (a bytea) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("bytes should not be supported"),
    };
}

#[rstest]
fn uuid_type(mut conn: PgConnection) {
    "CREATE TABLE t (a uuid) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES ('1bc6e67b-17f6-4bcc-a492-f0afb5212f38')".execute(&mut conn);
    let row: (Uuid,) = "SELECT * FROM t".fetch_one(&mut conn);

    assert_eq!(
        row.0,
        Uuid::try_parse("1bc6e67b-17f6-4bcc-a492-f0afb5212f38").unwrap()
    );
}

#[rstest]
fn oid_type(mut conn: PgConnection) {
    match "CREATE TABLE t (a oid) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("oid should not be supported"),
    };
}

#[rstest]
fn json_type(mut conn: PgConnection) {
    match "CREATE TABLE t (a json) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("json should not be supported"),
    };
}

#[rstest]
fn jsonb_type(mut conn: PgConnection) {
    match "CREATE TABLE t (a jsonb) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("jsonb should not be supported"),
    };
}

#[rstest]
fn timetz_type(mut conn: PgConnection) {
    match "CREATE TABLE t (a timetz) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("timetz should not be supported"),
    };
}
