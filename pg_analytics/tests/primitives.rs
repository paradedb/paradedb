mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::{types::BigDecimal, PgConnection};
use std::str::FromStr;
use time::{macros::format_description, Date, PrimitiveDateTime, Time};

#[rstest]
fn text_type(mut conn: PgConnection) {
    "CREATE TABLE test_text (a text) USING parquet".execute(&mut conn);
    "INSERT INTO test_text VALUES ('hello world')".execute(&mut conn);
    let row: (String,) = "SELECT * FROM test_text".fetch_one(&mut conn);
    assert_eq!(row.0, "hello world".to_string());
}

#[rstest]
fn varchar_type(mut conn: PgConnection) {
    "CREATE TABLE test_varchar (a varchar) USING parquet".execute(&mut conn);
    "INSERT INTO test_varchar VALUES ('hello world')".execute(&mut conn);
    let row: (String,) = "SELECT * FROM test_varchar".fetch_one(&mut conn);
    assert_eq!(row.0, "hello world".to_string());
}

#[rstest]
fn char_type(mut conn: PgConnection) {
    "CREATE TABLE test_char (a char) USING parquet".execute(&mut conn);
    "INSERT INTO test_char VALUES ('h')".execute(&mut conn);
    let row: (String,) = "SELECT * FROM test_char".fetch_one(&mut conn);
    assert_eq!(row.0, "h".to_string());
}

#[rstest]
fn smallint_type(mut conn: PgConnection) {
    "CREATE TABLE test_smallint (a smallint) USING parquet".execute(&mut conn);
    "INSERT INTO test_smallint VALUES (1)".execute(&mut conn);
    let row: (i16,) = "SELECT * FROM test_smallint".fetch_one(&mut conn);
    assert_eq!(row.0, 1);
}

#[rstest]
fn integer_type(mut conn: PgConnection) {
    "CREATE TABLE test_integer (a integer) USING parquet".execute(&mut conn);
    "INSERT INTO test_integer VALUES (1)".execute(&mut conn);
    let row: (i32,) = "SELECT * FROM test_integer".fetch_one(&mut conn);
    assert_eq!(row.0, 1);
}

#[rstest]
fn bigint_type(mut conn: PgConnection) {
    "CREATE TABLE test_bigint (a bigint) USING parquet".execute(&mut conn);
    "INSERT INTO test_bigint VALUES (1)".execute(&mut conn);
    let row: (i64,) = "SELECT * FROM test_bigint".fetch_one(&mut conn);
    assert_eq!(row.0, 1);
}

#[rstest]
fn real_type(mut conn: PgConnection) {
    "CREATE TABLE test_real (a real) USING parquet".execute(&mut conn);
    "INSERT INTO test_real VALUES (1.0)".execute(&mut conn);
    let row: (f32,) = "SELECT * FROM test_real".fetch_one(&mut conn);
    assert_eq!(row.0, 1.0);
}

#[rstest]
fn double_type(mut conn: PgConnection) {
    "CREATE TABLE test_double (a double precision) USING parquet".execute(&mut conn);
    "INSERT INTO test_double VALUES (1.0)".execute(&mut conn);
    let row: (f64,) = "SELECT * FROM test_double".fetch_one(&mut conn);
    assert_eq!(row.0, 1.0);
}

#[rstest]
fn bool_type(mut conn: PgConnection) {
    "CREATE TABLE test_bool (a bool) USING parquet".execute(&mut conn);
    "INSERT INTO test_bool VALUES (true)".execute(&mut conn);
    let row: (bool,) = "SELECT * FROM test_bool".fetch_one(&mut conn);
    assert_eq!(row.0, true);
}

#[rstest]
fn numeric_type(mut conn: PgConnection) {
    "CREATE TABLE test_numeric (a numeric(5, 2)) USING parquet".execute(&mut conn);
    "INSERT INTO test_numeric VALUES (1.01)".execute(&mut conn);
    let row: (BigDecimal,) = "SELECT * FROM test_numeric".fetch_one(&mut conn);
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
    "CREATE TABLE test_date (a date) USING parquet".execute(&mut conn);
    "INSERT INTO test_date VALUES ('2024-01-29')".execute(&mut conn);
    let row: (Date,) = "SELECT * FROM test_date".fetch_one(&mut conn);
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
    match "CREATE TABLE t (a uuid) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("uuid should not be supported"),
    };
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
