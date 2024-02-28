mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::{types::BigDecimal, PgConnection};
use std::str::FromStr;
use time::{macros::format_description, Date, PrimitiveDateTime};

#[rstest]
fn basic_select(mut conn: PgConnection) {
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
fn array_results(mut conn: PgConnection) {
    ResearchProjectArraysTable::setup().execute(&mut conn);

    let columns: Vec<ResearchProjectArraysTable> =
        "SELECT * FROM research_project_arrays".fetch_collect(&mut conn);

    // Using defaults for fields below that are unimplemented.
    let first = ResearchProjectArraysTable {
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

    let second = ResearchProjectArraysTable {
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

    assert_eq!(columns[0], first);
    assert_eq!(columns[1], second);
}

#[rstest]
fn alter(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);

    let rows: Vec<(String,)> = "SELECT column_name FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 't'".fetch(&mut conn);
    let column_names: Vec<_> = rows.into_iter().map(|r| r.0).collect();

    assert_eq!(column_names, vec!["a".to_string(), "b".to_string()]);

    "ALTER TABLE t ADD COLUMN c int".execute(&mut conn);

    let rows: Vec<(String,)> = "SELECT column_name FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 't'".fetch(&mut conn);
    let column_names: Vec<_> = rows.into_iter().map(|r| r.0).collect();

    assert_eq!(
        column_names,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[rstest]
#[ignore = "known bug where results after delete are out of order"]
fn delete(mut conn: PgConnection) {
    "CREATE TABLE employees (salary bigint, id smallint) USING parquet".execute(&mut conn);

    "INSERT INTO employees VALUES (100, 1), (200, 2), (300, 3), (400, 4), (500, 5)"
        .execute(&mut conn);
    "DELETE FROM employees WHERE id = 5 OR salary <= 200".execute(&mut conn);

    // TODO: Known bug here! The results are not in the correct order!
    let rows: Vec<(i64, i16)> = "SELECT * FROM employees".fetch(&mut conn);
    assert_eq!(rows, vec![(300, 3), (400, 4)]);
}

#[rstest]
fn drop(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);
    "DROP TABLE t".execute(&mut conn);

    match "SELECT * FROM t".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("relation 't' should not exist after drop"),
        Err(err) => assert!(err.to_string().contains("does not exist")),
    };

    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);
    "CREATE TABLE s (a int, b text)".execute(&mut conn);
    "DROP TABLE s, t".execute(&mut conn);

    match "SELECT * FROM s".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("relation 's' should not exist after drop"),
        Err(err) => assert!(err.to_string().contains("does not exist")),
    };

    match "SELECT * FROM t".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("relation 's' should not exist after drop"),
        Err(err) => assert!(err.to_string().contains("does not exist")),
    };
}

#[rstest]
fn insert(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b int)".execute(&mut conn);
    "INSERT INTO t VALUES (1, 2)".execute(&mut conn);
    "CREATE TABLE s (a int, b int) USING parquet".execute(&mut conn);
    "INSERT INTO s SELECT * FROM t".execute(&mut conn);

    let rows: Vec<(i32, i32)> = "SELECT * FROM s".fetch(&mut conn);
    assert_eq!(rows[0], (1, 2));
}

#[rstest]
fn join_two_parquet_tables(mut conn: PgConnection) {
    "CREATE TABLE t ( id INT PRIMARY KEY, name VARCHAR(50), department_id INT ) USING parquet"
        .execute(&mut conn);
    "CREATE TABLE s ( id INT PRIMARY KEY, department_name VARCHAR(50) ) USING parquet"
        .execute(&mut conn);

    r#"
    INSERT INTO t (id, name, department_id) VALUES
    (1, 'Alice', 101),
    (2, 'Bob', 102),
    (3, 'Charlie', 103),
    (4, 'David', 101);
    INSERT INTO s (id, department_name) VALUES
    (101, 'Human Resources'),
    (102, 'Finance'),
    (103, 'IT');
    "#
    .execute(&mut conn);

    let count: (i64,) =
        "SELECT COUNT(*) FROM t JOIN s ON t.department_id = s.id".fetch_one(&mut conn);
    assert_eq!(count, (4,));
}

#[rstest]
fn join_heap_and_parquet_table(mut conn: PgConnection) {
    "CREATE TABLE u ( id INT PRIMARY KEY, name VARCHAR(50), department_id INT ) USING parquet"
        .execute(&mut conn);
    "CREATE TABLE v ( id INT PRIMARY KEY, department_name VARCHAR(50) )".execute(&mut conn);
    r#"
    INSERT INTO u (id, name, department_id) VALUES
    (1, 'Alice', 101),
    (2, 'Bob', 102),
    (3, 'Charlie', 103),
    (4, 'David', 101);
    INSERT INTO v (id, department_name) VALUES
    (101, 'Human Resources'),
    (102, 'Finance'),
    (103, 'IT');
    "#
    .execute(&mut conn);

    match "SELECT COUNT(*) FROM u JOIN v ON u.department_id = v.id".fetch_result::<()>(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not yet supported")),
        _ => panic!("heap and parquet tables in same query should be unsupported"),
    }
}

#[rstest]
fn rename(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')".execute(&mut conn);
    "ALTER TABLE t RENAME TO s".execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT * FROM s".fetch(&mut conn);
    assert_eq!(rows[0], (1, "a".into()));
    assert_eq!(rows[1], (2, "b".into()));
    assert_eq!(rows[2], (3, "c".into()));
}

#[rstest]
fn schema(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text NOT NULL) USING parquet".execute(&mut conn);
    "INSERT INTO t values (1, 'test');".execute(&mut conn);

    let row: (i32, String) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row, (1, "test".into()));
}

#[rstest]
fn select(mut conn: PgConnection) {
    UserSessionLogsTable::setup().execute(&mut conn);

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
fn truncate(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);
    "INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c'); TRUNCATE t;".execute(&mut conn);

    let rows: Vec<(i32, String)> = "SELECT * FROM t".fetch(&mut conn);
    assert!(rows.is_empty())
}

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
        Err(err) => assert!(err.to_string().contains("not supported")),
        _ => panic!("bytes should not be supported"),
    };
    match "CREATE TABLE t (a uuid) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not supported")),
        _ => panic!("uuid should not be supported"),
    };
    match "CREATE TABLE t (a oid) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not supported")),
        _ => panic!("oid should not be supported"),
    };
    match "CREATE TABLE t (a json) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not supported")),
        _ => panic!("json should not be supported"),
    };
    match "CREATE TABLE t (a jsonb) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not supported")),
        _ => panic!("jsonb should not be supported"),
    };
    match "CREATE TABLE t (a time) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not supported")),
        _ => panic!("time should not be supported"),
    };
    match "CREATE TABLE t (a timetz) USING parquet".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("not supported")),
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
    ResearchProjectArraysTable::setup().execute(&mut conn);

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
    UserSessionLogsTable::setup().execute(&mut conn);

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
fn add_column(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);

    match "ALTER TABLE t ADD COLUMN a int".execute_result(&mut conn) {
        Err(err) => assert_eq!(
            err.to_string(),
            "error returned from database: column \"a\" of relation \"t\" already exists"
        ),
        _ => panic!("Adding a column with the same name should not be supported"),
    };

    "ALTER TABLE t ADD COLUMN c int".execute(&mut conn);
    "INSERT INTO t VALUES (1, 'a', 2)".execute(&mut conn);
    let row: (i32, String, i32) = "SELECT * FROM t".fetch_one(&mut conn);
    assert_eq!(row, (1, "a".into(), 2));
}

#[rstest]
fn drop_column(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text, c int) USING parquet".execute(&mut conn);

    match "ALTER TABLE t DROP COLUMN a".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: DROP COLUMN is not yet supported. Please recreate the table instead."),
        _ => panic!("Dropping a column should not be supported"),
    };
}

#[rstest]
fn rename_column(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);

    match "ALTER TABLE t RENAME COLUMN a TO c".execute_result(&mut conn) {
        Err(err) => assert_eq!(err.to_string(), "error returned from database: RENAME COLUMN is not yet supported. Please recreate the table instead."),
        _ => panic!("Renaming a column should not be supported"),
    };
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

    "CREATE TABLE test_table (id smallint) USING parquet; ALTER TABLE test_table ADD COLUMN name text; ALTER TABLE test_table ADD COLUMN age smallint;"
        .execute(&mut conn);
    let rows: Vec<(String,)> =
        "SELECT column_name FROM information_schema.columns WHERE table_name = 'test_table'"
            .fetch(&mut conn);
    let mut column_names: Vec<_> = rows.into_iter().map(|r| r.0).collect();
    assert_eq!(
        column_names.sort(),
        ["id".to_string(), "age".to_string(), "name".to_string()].sort()
    );

    "CREATE TABLE test_table2 (id smallint) USING parquet; INSERT INTO test_table2 VALUES (1), (2), (3); ALTER TABLE test_table2 ADD COLUMN name text;"
        .execute(&mut conn);
    let count: (i64,) = "SELECT COUNT(*) FROM test_table2".fetch_one(&mut conn);
    assert_eq!(count, (3,));
    let rows: Vec<(String,)> =
        "SELECT column_name FROM information_schema.columns WHERE table_name = 'test_table2'"
            .fetch(&mut conn);
    let mut column_names: Vec<_> = rows.into_iter().map(|r| r.0).collect();
    assert_eq!(
        column_names.sort(),
        ["id".to_string(), "name".to_string()].sort()
    );

    "CREATE TABLE test_table3 (id smallint) USING parquet; ALTER TABLE test_table3 ADD COLUMN name text; TRUNCATE TABLE test_table3;"
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
fn search_path(mut conn: PgConnection) {
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

#[rstest]
fn sqlparser_error(mut conn: PgConnection) {
    // Makes sure that statements like ALTER TABLE <table> ENABLE ROW LEVEL SECURITY
    // which are not supported by sqlparser are passed to Postgres successfully
    r#"
        DROP ROLE IF EXISTS engineering;
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
fn big_insert(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (
            id INT
        ) USING parquet;
        INSERT INTO t (id) SELECT generate_series(1, 100000);
        INSERT INTO t (id) SELECT generate_series(1, 100000);
    "#
    .execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (200000,));
}
