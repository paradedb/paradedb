#![allow(clippy::get_first)]
mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn explain_df_query(mut conn: PgConnection) {
    "CREATE TABLE t ( id INT PRIMARY KEY, name VARCHAR(50), department_id INT ) USING parquet"
        .execute(&mut conn);

    let res = "EXPLAIN SELECT * FROM t LIMIT 5;".fetch_scalar::<String>(&mut conn);

    assert_eq!(res.get(0).unwrap(), "Limit: skip=0, fetch=5");
    assert_eq!(
        res.get(1).unwrap(),
        "  Projection: t.id, t.name, t.department_id, t.parade_ctid, t.parade_xmin, t.parade_xmax"
    );
    assert_eq!(res.get(2).unwrap(), "    TableScan: t");
}

#[rstest]
fn explain_heap_query(mut conn: PgConnection) {
    "CREATE TABLE t ( id INT PRIMARY KEY, name VARCHAR(50), department_id INT )".execute(&mut conn);

    let res = "EXPLAIN SELECT * FROM t LIMIT 5;".fetch_scalar::<String>(&mut conn);

    assert_eq!(
        res.get(0).unwrap(),
        "Limit  (cost=0.00..0.14 rows=5 width=126)"
    );
    assert_eq!(
        res.get(1).unwrap(),
        "  ->  Seq Scan on t  (cost=0.00..15.30 rows=530 width=126)"
    );
}

#[rstest]
fn explain_federated_query(mut conn: PgConnection) {
    "CREATE TABLE t ( id INT PRIMARY KEY, name VARCHAR(50), department_id INT )".execute(&mut conn);
    "CREATE TABLE u ( id INT PRIMARY KEY, t_id INT ) USING parquet".execute(&mut conn);

    let res =
        "EXPLAIN SELECT * FROM t INNER JOIN u ON t.id = u.t_id;".fetch_scalar::<String>(&mut conn);

    assert_eq!(
        res.get(0).unwrap(),
        "Projection: t.id, t.name, t.department_id, u.id, u.t_id"
    );
    assert_eq!(res.get(1).unwrap(), "  Inner Join:  Filter: t.id = u.t_id");
    assert_eq!(res.get(2).unwrap(), "    TableScan: t");
    assert_eq!(res.get(3).unwrap(), "    TableScan: u");
}
