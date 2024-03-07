mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn join_two_heap_tables(mut conn: PgConnection) {
    "CREATE TABLE t ( id INT PRIMARY KEY, name VARCHAR(50), department_id INT )".execute(&mut conn);
    "CREATE TABLE s ( id INT PRIMARY KEY, department_name VARCHAR(50) )".execute(&mut conn);

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

    let count: (i64,) =
        "SELECT COUNT(*) FROM t JOIN s ON t.department_id = s.id".fetch_one(&mut conn);
    assert_eq!(count, (4,));
}
