mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn drop_parquet(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);
    "DROP TABLE t".execute(&mut conn);

    match "SELECT * FROM t".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("relation 't' should not exist after drop"),
        Err(err) => assert!(err.to_string().contains("does not exist")),
    };

    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);
    "CREATE TABLE s (a int, b text) USING parquet".execute(&mut conn);
    "DROP TABLE s, t".execute(&mut conn);

    match "SELECT * FROM s".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("relation 's' should not exist after drop"),
        Err(err) => assert!(err.to_string().contains("does not exist")),
    };

    match "SELECT * FROM t".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("relation 't' should not exist after drop"),
        Err(err) => assert!(err.to_string().contains("does not exist")),
    };
}

#[rstest]
fn drop_heap(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text)".execute(&mut conn);
    "CREATE TABLE s (a int, b text)".execute(&mut conn);
    "DROP TABLE s, t".execute(&mut conn);

    match "SELECT * FROM s".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("relation 's' should not exist after drop"),
        Err(err) => assert!(err.to_string().contains("does not exist")),
    };

    match "SELECT * FROM t".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("relation 't' should not exist after drop"),
        Err(err) => assert!(err.to_string().contains("does not exist")),
    };
}

#[rstest]
fn drop_heap_and_parquet(mut conn: PgConnection) {
    "CREATE TABLE t (a int, b text) USING parquet".execute(&mut conn);
    "CREATE TABLE s (a int, b text)".execute(&mut conn);
    "DROP TABLE s, t".execute(&mut conn);

    match "SELECT * FROM s".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("relation 's' should not exist after drop"),
        Err(err) => assert!(err.to_string().contains("does not exist")),
    };

    match "SELECT * FROM t".fetch_result::<()>(&mut conn) {
        Ok(_) => panic!("relation 't' should not exist after drop"),
        Err(err) => assert!(err.to_string().contains("does not exist")),
    };
}
