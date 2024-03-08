mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

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
