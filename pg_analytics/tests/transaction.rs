mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;
use std::path::Path;

#[rstest]
fn insert_transaction_aborted(mut conn: PgConnection) {
    r#"
        BEGIN;
        CREATE TABLE t (id INT PRIMARY KEY, name TEXT) USING parquet;  
        INSERT INTO t VALUES (1, 'test'), (2, 'test');      
    "#
    .execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (2,));

    r#"
        INSERT INTO t VALUES (3, 'test'), (4, 'test'); 
        INSERT INTO t VALUES (5, 'test'), (6, 'test');   
    "#
    .execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (6,));

    match "INSERT INTO t VALUES (1, 'test')".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains(
            "error returned from database: duplicate key value violates unique constraint"
        )),
        _ => panic!("Primary key constraint violated"),
    };

    match "INSERT INTO t VALUES (1, 'test')".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("current transaction is aborted")),
        _ => panic!("Transaction should have been aborted"),
    };

    "ROLLBACK".execute(&mut conn);

    match "SELECT COUNT(*) FROM t".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("does not exist")),
        _ => panic!("Table should not exist"),
    };
}

#[rstest]
fn insert_transaction_commit(mut conn: PgConnection) {
    "CREATE TABLE t (id INT PRIMARY KEY, name TEXT) USING parquet".execute(&mut conn);

    r#"
        BEGIN;
        INSERT INTO t VALUES (1, 'test'), (2, 'test');
        INSERT INTO t VALUES (3, 'test'), (4, 'test');
        COMMIT;        
    "#
    .execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (4,));
}

#[rstest]
fn insert_transaction_rollback(mut conn: PgConnection) {
    "CREATE TABLE t (id INT PRIMARY KEY, name TEXT) USING parquet".execute(&mut conn);

    r#"
        BEGIN;
        INSERT INTO t VALUES (1, 'test'), (2, 'test');
        INSERT INTO t VALUES (3, 'test'), (4, 'test');
    "#
    .execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (4,));

    "ROLLBACK".execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (0,));
}

#[rstest]
fn truncate_transaction_commit(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (id INT PRIMARY KEY, name TEXT) USING parquet;
        INSERT INTO t VALUES (1, 'test'), (2, 'test');
        INSERT INTO t VALUES (3, 'test'), (4, 'test');
        BEGIN;
        TRUNCATE t;
        COMMIT;      
    "#
    .execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (0,));
}

#[rstest]
fn truncate_transaction_rollback(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (id INT PRIMARY KEY, name TEXT) USING parquet;
        INSERT INTO t VALUES (1, 'test'), (2, 'test');
        INSERT INTO t VALUES (3, 'test'), (4, 'test');     
    "#
    .execute(&mut conn);

    r#"
        BEGIN;
        TRUNCATE t; 
    "#
    .execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (0,));

    "ROLLBACK".execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (4,));
}

#[rstest]
fn drop_transaction_commit(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (id INT PRIMARY KEY, name TEXT) USING parquet;
        INSERT INTO t VALUES (1, 'test'), (2, 'test');  
    "#
    .execute(&mut conn);

    let table_path = default_table_path(&mut conn, "public", "t");

    r#"
        BEGIN;
        DROP TABLE t;
        COMMIT;      
    "#
    .execute(&mut conn);

    match "SELECT COUNT(*) FROM t".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("does not exist")),
        _ => panic!("Table should not exist"),
    };

    assert!(!table_path.exists());

    r#"
        BEGIN;
        CREATE TABLE t (id INT PRIMARY KEY, name TEXT) USING parquet;
        INSERT INTO t VALUES (1, 'test'), (2, 'test');
        DROP TABLE t;
        COMMIT;      
    "#
    .execute(&mut conn);

    match "SELECT COUNT(*) FROM t".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("does not exist")),
        _ => panic!("Table should not exist"),
    };

    assert!(!table_path.exists());
}

#[rstest]
fn drop_transaction_rollback(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (id INT PRIMARY KEY, name TEXT) USING parquet;
        INSERT INTO t VALUES (1, 'test'), (2, 'test');
    "#
    .execute(&mut conn);

    let first_table_path = default_table_path(&mut conn, "public", "t")
        .to_str()
        .unwrap()
        .to_string();

    r#"
        BEGIN;
        DROP TABLE t;
    "#
    .execute(&mut conn);

    match "SELECT COUNT(*) FROM t".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("does not exist")),
        _ => panic!("Table should not exist"),
    };

    assert!(Path::new(&first_table_path).exists());

    "ROLLBACK".execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM t".fetch_one(&mut conn);
    assert_eq!(count, (2,));

    "DROP TABLE t".execute(&mut conn);

    assert!(!Path::new(&first_table_path).exists());

    r#"
        BEGIN;
        CREATE TABLE t (id INT PRIMARY KEY, name TEXT) USING parquet;
        INSERT INTO t VALUES (1, 'test'), (2, 'test');
    "#
    .execute(&mut conn);

    let second_table_path = default_table_path(&mut conn, "public", "t")
        .to_str()
        .unwrap()
        .to_string();

    "DROP TABLE t".execute(&mut conn);
    match "SELECT COUNT(*) FROM t".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("does not exist")),
        _ => panic!("Table should not exist"),
    };

    assert!(Path::new(&second_table_path).exists());

    "ROLLBACK".execute(&mut conn);

    match "SELECT COUNT(*) FROM t".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("does not exist")),
        _ => panic!("Table should not exist"),
    };

    assert!(!Path::new(&second_table_path).exists());
}
