mod fixtures;
mod utils;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;
use utils::*;

#[rstest]
fn partition(mut conn: PgConnection) {
    r#"
        CREATE TABLE t (a int, b int, c int) PARTITION BY LIST(a,b) USING parquet;
        INSERT INTO t VALUES (1, 2, 3), (4, 5, 6), (7, 8, 9);
    "#
    .execute(&mut conn);

    let mut a_res: Vec<i32> = "SELECT a FROM t"
        .fetch(&mut conn)
        .into_iter()
        .map(|r: (i32,)| r.0)
        .collect();
    a_res.sort();
    assert_eq!(a_res, [1, 4, 7]);

    let mut b_res: Vec<i32> = "SELECT b FROM t"
        .fetch(&mut conn)
        .into_iter()
        .map(|r: (i32,)| r.0)
        .collect();
    b_res.sort();
    assert_eq!(b_res, [2, 5, 8]);

    let mut c_res: Vec<i32> = "SELECT c FROM t"
        .fetch(&mut conn)
        .into_iter()
        .map(|r: (i32,)| r.0)
        .collect();
    c_res.sort();
    assert_eq!(c_res, [3, 6, 9]);

    let data_path = test_data_path(&mut conn);
    let total_parquet_files = total_parquet_files_in_dir(&data_path);
    assert!(total_parquet_files == 3);
}

#[rstest]
fn partition_wrong_strategy(mut conn: PgConnection) {
    match "CREATE TABLE t (a int, b int, c int) PARTITION BY RANGE(a) USING parquet"
        .execute_result(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("PARTITION BY on parquet tables is only supported with LIST")),
        _ => panic!("Allowed unsupported PARTITION BY strategy"),
    };
}
