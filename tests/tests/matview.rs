mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn refresh_matview_concurrently_issue2308(mut conn: PgConnection) {
    // if this doesn't raise an ERROR then it worked
    r#"
    DROP MATERIALIZED VIEW IF EXISTS TEST_mv;
    DROP TABLE IF EXISTS TEST_tbl;

    -- 2) Setup table
    CREATE table TEST_tbl (
        id integer
    );

    -- 3) insert data (data is optional for it to fail)
    -- INSERT INTO TEST_1 VALUES (1), (2), (3), (4);

    -- 4) Setup materialized view
    CREATE MATERIALIZED VIEW TEST_mv AS (SELECT * FROM TEST_tbl);
    CREATE UNIQUE INDEX test_idx ON TEST_mv (id); -- required for `CONCURRENTLY` to work
    CREATE INDEX TEST_bm25 ON TEST_mv USING bm25 (id) WITH (key_field='id');

    -- 5) Refresh the view concurrently
    REFRESH MATERIALIZED VIEW CONCURRENTLY TEST_mv;
    "#
    .execute(&mut conn);
}
