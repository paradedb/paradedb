mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::{types::BigDecimal, PgConnection};
use std::str::FromStr;
use time::{macros::format_description, Date, PrimitiveDateTime};

#[rstest]
fn types(mut conn: PgConnection) {
    "CREATE TABLE test_text (a text) USING parquet".execute(&mut conn);
    "INSERT INTO test_text VALUES ('hello world')".execute(&mut conn);
    let row: (String,) = "SELECT * FROM test_text".fetch_one(&mut conn);
    assert_eq!(row.0, "hello world".to_string());

    "CREATE TABLE test_varchar (a varchar) USING parquet".execute(&mut conn);
    "INSERT INTO test_varchar VALUES ('hello world')".execute(&mut conn);
    let row: (String,) = "SELECT * FROM test_varchar".fetch_one(&mut conn);
    assert_eq!(row.0, "hello world".to_string());

    "CREATE TABLE test_char (a char) USING parquet".execute(&mut conn);
    "INSERT INTO test_char VALUES ('h')".execute(&mut conn);
    let row: (String,) = "SELECT * FROM test_char".fetch_one(&mut conn);
    assert_eq!(row.0, "h".to_string());

    "CREATE TABLE test_smallint (a smallint) USING parquet".execute(&mut conn);
    "INSERT INTO test_smallint VALUES (1)".execute(&mut conn);
    let row: (i16,) = "SELECT * FROM test_smallint".fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    "CREATE TABLE test_integer (a integer) USING parquet".execute(&mut conn);
    "INSERT INTO test_integer VALUES (1)".execute(&mut conn);
    let row: (i32,) = "SELECT * FROM test_integer".fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    "CREATE TABLE test_bigint (a bigint) USING parquet".execute(&mut conn);
    "INSERT INTO test_bigint VALUES (1)".execute(&mut conn);
    let row: (i64,) = "SELECT * FROM test_bigint".fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    "CREATE TABLE test_real (a real) USING parquet".execute(&mut conn);
    "INSERT INTO test_real VALUES (1.0)".execute(&mut conn);
    let row: (f32,) = "SELECT * FROM test_real".fetch_one(&mut conn);
    assert_eq!(row.0, 1.0);

    "CREATE TABLE test_double (a double precision) USING parquet".execute(&mut conn);
    "INSERT INTO test_double VALUES (1.0)".execute(&mut conn);
    let row: (f64,) = "SELECT * FROM test_double".fetch_one(&mut conn);
    assert_eq!(row.0, 1.0);

    "CREATE TABLE test_bool (a bool) USING parquet".execute(&mut conn);
    "INSERT INTO test_bool VALUES (true)".execute(&mut conn);
    let row: (bool,) = "SELECT * FROM test_bool".fetch_one(&mut conn);
    assert_eq!(row.0, true);

    "CREATE TABLE test_numeric (a numeric(5, 2)) USING parquet".execute(&mut conn);
    "INSERT INTO test_numeric VALUES (1.01)".execute(&mut conn);
    let row: (BigDecimal,) = "SELECT * FROM test_numeric".fetch_one(&mut conn);
    assert_eq!(row.0, BigDecimal::from_str("1.01").unwrap());

    "CREATE TABLE test_timestamp (a timestamp) USING parquet".execute(&mut conn);
    "INSERT INTO test_timestamp VALUES ('2024-01-29 15:30:00')".execute(&mut conn);
    let row: (PrimitiveDateTime,) = "SELECT * FROM test_timestamp".fetch_one(&mut conn);
    let fd = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    assert_eq!(
        row.0,
        PrimitiveDateTime::parse("2024-01-29 15:30:00", fd).unwrap()
    );

    "CREATE TABLE test_date (a date) USING parquet".execute(&mut conn);
    "INSERT INTO test_date VALUES ('2024-01-29')".execute(&mut conn);
    let row: (Date,) = "SELECT * FROM test_date".fetch_one(&mut conn);
    let fd = format_description!("[year]-[month]-[day]");
    assert_eq!(row.0, Date::parse("2024-01-29", fd).unwrap());

    match "CREATE TABLE t (a bytea) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("bytes should not be supported"),
    };
    match "CREATE TABLE t (a uuid) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("uuid should not be supported"),
    };
    match "CREATE TABLE t (a oid) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("oid should not be supported"),
    };
    match "CREATE TABLE t (a json) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("json should not be supported"),
    };
    match "CREATE TABLE t (a jsonb) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("jsonb should not be supported"),
    };
    match "CREATE TABLE t (a time) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("time should not be supported"),
    };
    match "CREATE TABLE t (a timetz) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("timetz should not be supported"),
    };
}

#[rstest]
fn vacuum(mut conn: PgConnection) {
    "CREATE TABLE t (a int) USING parquet".execute(&mut conn);
    "CREATE TABLE s (a int)".execute(&mut conn);
    "INSERT INTO t VALUES (1), (2), (3)".execute(&mut conn);
    "INSERT INTO s VALUES (4), (5), (6)".execute(&mut conn);
    "VACUUM".execute(&mut conn);
    "VACUUM FULL".execute(&mut conn);
    "VACUUM t".execute(&mut conn);
    "VACUUM FULL t".execute(&mut conn);
    "DROP TABLE t, s".execute(&mut conn);
    "VACUUM".execute(&mut conn);
}

#[rstest]
async fn copy_out_arrays(mut conn: PgConnection) {
    ResearchProjectArraysTable::setup_parquet().execute(&mut conn);

    let copied_csv = conn
        .copy_out_raw(
            "COPY (SELECT * FROM research_project_arrays) TO STDOUT WITH (FORMAT CSV, HEADER)",
        )
        .await
        .unwrap()
        .to_csv();

    let expected_csv = r#"
experiment_flags,notes,keywords,short_descriptions,participant_ages,participant_ids,observation_counts,measurement_errors,precise_measurements
"{t,f,t}","{""Initial setup complete"",""Preliminary results promising""}","{""climate change"",""coral reefs""}","{""CRLRST    "",""OCEAN1    ""}","{28,34,29}","{101,102,103}","{150,120,130}","{0.02,0.03,0.015}","{1.5,1.6,1.7}"
"{f,t,f}","{""Need to re-evaluate methodology"",""Unexpected results in phase 2""}","{""sustainable farming"",""soil health""}","{""FARMEX    "",""SOILQ2    ""}","{22,27,32}","{201,202,203}","{160,140,135}","{0.025,0.02,0.01}","{2,2.1,2.2}""#;

    assert_eq!(copied_csv.trim(), expected_csv.trim());
}

#[rstest]
async fn copy_out_basic(mut conn: PgConnection) {
    UserSessionLogsTable::setup_parquet().execute(&mut conn);

    let copied_csv = conn
        .copy_out_raw(
            "COPY (SELECT * FROM user_session_logs ORDER BY id) TO STDOUT WITH (FORMAT CSV, HEADER)",
        )
        .await
        .unwrap()
        .to_csv();

    let expected_csv = r#"
id,event_date,user_id,event_name,session_duration,page_views,revenue
1,2024-01-01,1,Login,300,5,20.00
2,2024-01-02,2,Purchase,450,8,150.50
3,2024-01-03,3,Logout,100,2,0.00
4,2024-01-04,4,Signup,200,3,0.00
5,2024-01-05,5,ViewProduct,350,6,30.75
6,2024-01-06,1,AddToCart,500,10,75.00
7,2024-01-07,2,RemoveFromCart,250,4,0.00
8,2024-01-08,3,Checkout,400,7,200.25
9,2024-01-09,4,Payment,550,11,300.00
10,2024-01-10,5,Review,600,9,50.00
11,2024-01-11,6,Login,320,3,0.00
12,2024-01-12,7,Purchase,480,7,125.30
13,2024-01-13,8,Logout,150,2,0.00
14,2024-01-14,9,Signup,240,4,0.00
15,2024-01-15,10,ViewProduct,360,5,45.00
16,2024-01-16,6,AddToCart,510,9,80.00
17,2024-01-17,7,RemoveFromCart,270,3,0.00
18,2024-01-18,8,Checkout,430,6,175.50
19,2024-01-19,9,Payment,560,12,250.00
20,2024-01-20,10,Review,610,10,60.00"#;

    assert_eq!(copied_csv.trim(), expected_csv.trim());
}

#[rstest]
fn multiline_query(mut conn: PgConnection) {
    "CREATE TABLE employees (salary bigint, id smallint) USING parquet; INSERT INTO employees VALUES (100, 1), (200, 2), (300, 3), (400, 4), (500, 5);".execute(&mut conn);
    let insert_count: (i64,) = "SELECT COUNT(*) FROM employees".fetch_one(&mut conn);
    assert_eq!(insert_count, (5,));

    if "SELECT COUNT(*) FROM employees; DELETE FROM employees WHERE id = 5 OR salary <= 200;"
        .execute_result(&mut conn)
        .is_err()
    {
        panic!("Multiline query with select and delete should not error out");
    };
    let select_count: (i64,) = "SELECT COUNT(*) FROM employees".fetch_one(&mut conn);
    assert_eq!(select_count, (2,));

    "CREATE TABLE test_table3 (id smallint) USING parquet; INSERT INTO test_table3 VALUES (1); TRUNCATE TABLE test_table3;"
        .execute(&mut conn);
    let count: (i64,) = "SELECT COUNT(*) FROM test_table3".fetch_one(&mut conn);
    assert_eq!(count, (0,));
    let rows: Vec<(String,)> =
        "SELECT column_name FROM information_schema.columns WHERE table_name = 'test_table3'"
            .fetch(&mut conn);
    let mut column_names: Vec<_> = rows.into_iter().map(|r| r.0).collect();
    assert_eq!(
        column_names.sort(),
        ["id".to_string(), "name".to_string()].sort()
    );
}

#[rstest]
fn sqlparser_error(mut conn: PgConnection) {
    // Makes sure that statements like ALTER TABLE <table> ENABLE ROW LEVEL SECURITY
    // which are not supported by sqlparser are passed to Postgres successfully
    r#"
        DO $$
        BEGIN
            IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'engineering') THEN
                EXECUTE 'DROP OWNED BY engineering CASCADE';
                EXECUTE 'DROP ROLE engineering';
            END IF;
        END$$;
        CREATE ROLE engineering LOGIN PASSWORD 'password';
        CREATE TABLE employee (
            id SERIAL PRIMARY KEY,
            name VARCHAR(100),
            salary INTEGER,
            department VARCHAR(100)
        );
        ALTER TABLE employee ENABLE ROW LEVEL SECURITY;
        CREATE POLICY select_department ON employee
        FOR SELECT
        TO public
        USING (department = current_user);
        INSERT INTO employee (name, salary, department) VALUES
            ('John Doe', 50000, 'engineering'),
            ('Jane Smith', 55000, 'hr'),
            ('Alice Johnson', 60000, 'engineering'),
            ('Bob Brown', 45000, 'marketing');
        GRANT SELECT ON TABLE employee TO engineering;
        SET ROLE engineering;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = "SELECT id FROM employee".fetch(&mut conn);
    let ids: Vec<i32> = rows.into_iter().map(|r| r.0).collect();
    assert_eq!(ids, [1, 3]);
}

#[rstest]
fn timestamp_unbounded(mut conn: PgConnection) {
    let timestamp_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let date_format = format_description!("[year]-[month]-[day]");

    r#"
        CREATE TABLE dates (
            date_column DATE,
            timestamp_column TIMESTAMP
        ) USING parquet;
    "#
    .execute(&mut conn);

    r#"
        INSERT INTO dates (date_column, timestamp_column)
        VALUES ('2022-01-01', '2022-01-01 12:00:00'),
               ('0001-02-02', '0001-02-02 12:00:00'),
               ('1230-02-03', '1230-02-03 12:00:00'),
               ('1230-02-04', '1230-02-04 12:00:00'),
               ('2000-02-04', '2000-02-04 12:00:00');
    "#
    .execute(&mut conn);

    let rows: Vec<(Date, PrimitiveDateTime)> = "SELECT * FROM dates".fetch(&mut conn);
    assert_eq!(rows[0].0, Date::parse("2022-01-01", date_format).unwrap());
    assert_eq!(rows[1].0, Date::parse("0001-02-02", date_format).unwrap());
    assert_eq!(rows[2].0, Date::parse("1230-02-03", date_format).unwrap());
    assert_eq!(rows[3].0, Date::parse("1230-02-04", date_format).unwrap());
    assert_eq!(rows[4].0, Date::parse("2000-02-04", date_format).unwrap());
    assert_eq!(
        rows[0].1,
        PrimitiveDateTime::parse("2022-01-01 12:00:00", timestamp_format).unwrap()
    );
    assert_eq!(
        rows[1].1,
        PrimitiveDateTime::parse("0001-02-02 12:00:00", timestamp_format).unwrap()
    );
    assert_eq!(
        rows[2].1,
        PrimitiveDateTime::parse("1230-02-03 12:00:00", timestamp_format).unwrap()
    );
    assert_eq!(
        rows[3].1,
        PrimitiveDateTime::parse("1230-02-04 12:00:00", timestamp_format).unwrap()
    );
    assert_eq!(
        rows[4].1,
        PrimitiveDateTime::parse("2000-02-04 12:00:00", timestamp_format).unwrap()
    );

    let rows: Vec<(Date, PrimitiveDateTime)> =
        "SELECT * FROM dates WHERE timestamp_column < '2000-02-04 12:00:01'::timestamp"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 4);

    let rows: Vec<(Date, PrimitiveDateTime)> =
        "SELECT * FROM dates WHERE timestamp_column < '2000-02-04 12:00:00'::timestamp"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 3);

    let rows: Vec<(Date, PrimitiveDateTime)> =
        "SELECT * FROM dates WHERE date_column = '2000-02-04'::date".fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    let rows: Vec<(Date, PrimitiveDateTime)> =
        "SELECT * FROM dates WHERE date_column < '1230-02-04'::date".fetch(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(Date, PrimitiveDateTime)> =
        "SELECT * FROM dates WHERE date_column < '1230-02-03'::date".fetch(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn timestamp_precision(mut conn: PgConnection) {
    r#"
        CREATE TABLE timestamps (
            timestamp_6 TIMESTAMP(6)
        ) USING parquet;
    "#
    .execute(&mut conn);

    r#"
        INSERT INTO timestamps (timestamp_6)
        VALUES ('2022-01-01 12:00:00.123456'),
               ('2022-02-02 12:00:00.456789');
    "#
    .execute(&mut conn);

    let rows: Vec<(PrimitiveDateTime,)> = "SELECT * FROM timestamps".fetch(&mut conn);
    assert_eq!(
        rows[0].0,
        PrimitiveDateTime::parse(
            "2022-01-01 12:00:00",
            format_description!("[year]-[month]-[day] [hour]:[minute]:[second]")
        )
        .unwrap()
    );
    assert_eq!(
        rows[1].0,
        PrimitiveDateTime::parse(
            "2022-02-02 12:00:00",
            format_description!("[year]-[month]-[day] [hour]:[minute]:[second]")
        )
        .unwrap()
    );

    let count: (i64,) =
        "SELECT COUNT(*) FROM timestamps where timestamp_6 < '2022-02-02 12:00:00.456788'::timestamp"
            .fetch_one(&mut conn);
    assert_eq!(count, (1,));

    let count: (i64,) =
        "SELECT COUNT(*) FROM timestamps where timestamp_6 < '2022-02-02 12:00:00.456790'::timestamp"
            .fetch_one(&mut conn);
    assert_eq!(count, (2,));

    match "CREATE TABLE s (a timestamp(3)) USING parquet".fetch_result::<()>(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: Schema error: Invalid data type for Delta Lake: Timestamp(Millisecond, None)"),
        _ => panic!("timestamp(3) should not be supported"),
    }

    match "CREATE TABLE s (a timestamp(2)) USING parquet".fetch_result::<()>(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: Only timestamp and timestamp(6), not timestamp(2), are supported"),
        _ => panic!("timestamp(3) should not be supported"),
    }
}

#[rstest]
fn numeric(mut conn: PgConnection) {
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
    assert_eq!(rows[0].0, BigDecimal::from_str("12.34").unwrap());
    assert_eq!(rows[0].1, BigDecimal::from_str("123.67890").unwrap());
    assert_eq!(rows[0].2, BigDecimal::from_str("1234.1234567890").unwrap());
    assert_eq!(rows[1].0, BigDecimal::from_str("123.34").unwrap());
    assert_eq!(rows[1].1, BigDecimal::from_str("123.45678").unwrap());
    assert_eq!(rows[1].2, BigDecimal::from_str("12345.2234567890").unwrap());

    let rows: Vec<(BigDecimal, BigDecimal, BigDecimal)> =
        "SELECT * FROM t WHERE num2 = 123.45678".fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    match "CREATE TABLE s (num1 NUMERIC) USING parquet".fetch_result::<()>(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("unbounded numerics should not be supported"),
    }
}
