// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

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
