mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::{types::BigDecimal, PgConnection};
use std::str::FromStr;

#[rstest]
fn numeric_with_typemod(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (
            num1 NUMERIC(5, 2),
            num2 NUMERIC(10, 5),
            num3 NUMERIC(15, 10)
        ) USING parquet;
    "#
    .execute(&mut conn);

    r#"
        INSERT INTO t (num1, num2, num3)
        VALUES (12.34, 123.67890, 1234.1234567890),
               (123.34, 123.45678, 12345.2234567890);
    "#
    .execute(&mut conn);

    let rows: Vec<(BigDecimal, BigDecimal, BigDecimal)> = "SELECT * FROM t".fetch(&mut conn);

    let first_column = vec![
        BigDecimal::from_str("12.34").unwrap(),
        BigDecimal::from_str("123.34").unwrap(),
    ];
    let second_column = vec![
        BigDecimal::from_str("123.67890").unwrap(),
        BigDecimal::from_str("123.45678").unwrap(),
    ];
    let third_column = vec![
        BigDecimal::from_str("1234.1234567890").unwrap(),
        BigDecimal::from_str("12345.2234567890").unwrap(),
    ];

    assert!(rows.iter().map(|r| r.0.clone()).eq(first_column));
    assert!(rows.iter().map(|r| r.1.clone()).eq(second_column));
    assert!(rows.iter().map(|r| r.2.clone()).eq(third_column));

    let count: (i64,) = "SELECT COUNT(*) FROM t WHERE num2 = 123.45678".fetch_one(&mut conn);
    assert_eq!(count, (1,));
}

#[rstest]
fn numeric_without_typemod(mut conn: PgConnection) {
    match "CREATE TABLE s (num1 NUMERIC) USING parquet".fetch_result::<()>(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("unbounded numerics should not be supported"),
    }
}
