mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn wal_identify(mut conn_with_walinspect: PgConnection) {
    if pg_version(&mut conn_with_walinspect) < 15 {
        return;
    }

    let first_lsn = current_lsn(&mut conn_with_walinspect);

    r#"
        CREATE TABLE t (a int) USING parquet;
        CREATE TABLE s (a int) USING parquet;
        CREATE TABLE u (a int);
        INSERT INTO t VALUES (1), (2);
        INSERT INTO s VALUES (4), (5);
        INSERT INTO u VALUES (7), (8);
        TRUNCATE s, t, u;
    "#
    .execute(&mut conn_with_walinspect);

    let second_lsn = current_lsn(&mut conn_with_walinspect);

    let rows: Vec<(String,)> = format!(
        r#"
            SELECT record_type from pg_get_wal_records_info('{}'::pg_lsn, '{}'::pg_lsn) 
            WHERE resource_manager = 'pg_analytics'
        "#,
        first_lsn, second_lsn
    )
    .fetch(&mut conn_with_walinspect);

    let records = vec![
        "INSERT", "INSERT", "INSERT", "INSERT", "TRUNCATE", "TRUNCATE",
    ];
    assert!(rows.iter().map(|r| r.0.clone()).eq(records));
}
