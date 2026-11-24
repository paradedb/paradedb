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

use anyhow::Result;
use rstest::*;
use sqlx::{AssertSqlSafe, Executor};
use tests::fixtures::*;

// Two bulk inserts per table give each index two segments, which an MPP launch needs.
const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CREATE TABLE reload_users (id bigserial PRIMARY KEY, name text, age int);
CREATE TABLE reload_products (id bigserial PRIMARY KEY, name text, age int);
CREATE INDEX reload_users_idx ON reload_users USING paradedb (id, (name::pdb.literal), age);
CREATE INDEX reload_products_idx ON reload_products USING paradedb (id, (name::pdb.literal), age);

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO reload_users (name, age)
SELECT (ARRAY['bob', 'alice'])[1 + (g % 2)], g % 50 FROM generate_series(1, 5000) g;
INSERT INTO reload_users (name, age)
SELECT (ARRAY['bob', 'alice'])[1 + (g % 2)], g % 50 FROM generate_series(5001, 10000) g;
INSERT INTO reload_products (name, age) SELECT 'x', g % 50 FROM generate_series(1, 5000) g;
INSERT INTO reload_products (name, age) SELECT 'x', g % 50 FROM generate_series(5001, 10000) g;
RESET paradedb.global_mutable_segment_rows;
ANALYZE reload_users;
ANALYZE reload_products;
"#;

const MPP_GUCS: &str = r#"
SET paradedb.enable_join_custom_scan TO on;
-- The fixture tables are tiny. Without this, the size gate keeps MPP off.
SET paradedb.mpp_min_rows TO 0;
SET max_parallel_workers_per_gather TO 2;
SET max_parallel_workers TO 2;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
"#;

// The outer query asks for a configuration reload and then sleeps, so the reload is pending in
// this backend when `Execute` returns. The backend applies it before the next protocol message.
const MPP_QUERY: &str = r#"
SELECT s.id, pg_reload_conf(), pg_sleep(0.2) IS NOT NULL
FROM (
    SELECT u.id
    FROM reload_users u
    JOIN reload_products p ON u.age = p.age
    WHERE u.name ||| 'bob'
    ORDER BY u.id
    LIMIT 2
) s
"#;

#[rstest]
#[tokio::test]
async fn mpp_join_survives_config_reload(database: Db) -> Result<()> {
    let mut conn = database.connection().await;
    conn.execute(SETUP_SQL).await?;
    conn.execute(MPP_GUCS).await?;

    let plan = sqlx::query_as::<_, (String,)>(AssertSqlSafe(format!("EXPLAIN {MPP_QUERY}")))
        .fetch_all(&mut conn)
        .await?
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        plan.contains("DistributedExec"),
        "expected the join to launch MPP workers, got:\n{plan}"
    );

    // `query_as` uses the extended protocol. The portal of the query stays open after `Execute`,
    // until the backend reads `Close` or `Sync`.
    let rows: Vec<(i64, bool, bool)> = sqlx::query_as(MPP_QUERY).fetch_all(&mut conn).await?;
    assert_eq!(rows, vec![(2, true, true), (2, true, true)]);

    Ok(())
}
