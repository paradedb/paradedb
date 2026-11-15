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

//! End-to-end verification of cross-stage distributed dynamic filters in MPP (`DistributedExec`).
//!
//! Covers all four dynamic filter producer families across network stage boundaries:
//! 1. Shuffled `HashJoinExec` (`mode=Partitioned`, `UnionWhenAllComplete` merge policy)
//! 2. Cross-stage `SortExec: TopK` (`IntersectOnUpdate` merge policy)
//! 3. Cross-stage `SegmentedTopKExec` on deferred dictionary text columns (`IntersectOnUpdate`
//!    via `dynamic_expressions_produced()` + `fetch()`)
//! 4. Cross-stage `AggregateExec` (`mode=Partial`, `MIN`, `IntersectOnUpdate` merge policy)

use anyhow::{Context, Result};
use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
use tests::fixtures::*;

// `df_files` (1,500 rows across 3 flushed segments) is smaller than `df_pages` (9,000 rows across
// 3 flushed segments), and `df_pages.file_id` is non-unique (6 pages per file). Consequently:
// - `WHERE f.id IN (SELECT p.file_id FROM df_pages p ...)` plans as `f SEMI JOIN p` with `f` on
//   the build side (`LeftSemi`), which `normalize_collect_joins` converts to
//   `HashJoinExec(mode=Partitioned)` over `NetworkShuffleExec` on both sides.
// - `C` collation on `title` enables byte-ordered dictionary `SegmentedTopKExec` regardless of
//   cluster locale.
const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CREATE TABLE df_files (
    id INTEGER PRIMARY KEY,
    score_bucket INTEGER NOT NULL,
    title TEXT COLLATE "C" NOT NULL,
    content TEXT NOT NULL
);

CREATE TABLE df_pages (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    page_text TEXT NOT NULL
);

CREATE INDEX df_files_idx ON df_files
USING paradedb (
    id,
    score_bucket,
    (title::pdb.literal),
    content
) WITH (layer_sizes = '0', background_layer_sizes = '0');

CREATE INDEX df_pages_idx ON df_pages
USING paradedb (
    id,
    file_id,
    size_bytes,
    page_text
) WITH (layer_sizes = '0', background_layer_sizes = '0');

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO df_files (id, score_bucket, title, content)
SELECT g,
       (g * 37) % 500,
       'file-' || lpad(g::text, 6, '0'),
       'section ' || g::text || ' testing dynamic filters'
FROM generate_series(1, 500) AS g;

INSERT INTO df_files (id, score_bucket, title, content)
SELECT g,
       (g * 37) % 500,
       'file-' || lpad(g::text, 6, '0'),
       'section ' || g::text || ' testing dynamic filters'
FROM generate_series(501, 1000) AS g;

INSERT INTO df_files (id, score_bucket, title, content)
SELECT g,
       (g * 37) % 500,
       'file-' || lpad(g::text, 6, '0'),
       'section ' || g::text || ' testing dynamic filters'
FROM generate_series(1001, 1500) AS g;

INSERT INTO df_pages (id, file_id, size_bytes, page_text)
SELECT g,
       ((g - 1) % 1500) + 1,
       (g * 17) % 4096,
       'page ' || g::text || ' body text'
FROM generate_series(1, 3000) AS g;

INSERT INTO df_pages (id, file_id, size_bytes, page_text)
SELECT g,
       ((g - 1) % 1500) + 1,
       (g * 17) % 4096,
       'page ' || g::text || ' body text'
FROM generate_series(3001, 6000) AS g;

INSERT INTO df_pages (id, file_id, size_bytes, page_text)
SELECT g,
       ((g - 1) % 1500) + 1,
       (g * 17) % 4096,
       'page ' || g::text || ' body text'
FROM generate_series(6001, 9000) AS g;

RESET paradedb.global_mutable_segment_rows;

ANALYZE df_files;
ANALYZE df_pages;
"#;

const COMMON_GUCS: &str = r#"
SET paradedb.enable_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.mpp_min_rows TO 0;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET min_parallel_index_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
"#;

#[derive(Clone, Copy, Debug)]
enum CrossStageDynamicFilterCase {
    ShuffledHashJoin,
    CrossStageSortTopK,
    CrossStageSegmentedTopK,
    CrossStageAggregateMinMax,
}

impl CrossStageDynamicFilterCase {
    const ALL: [Self; 4] = [
        Self::ShuffledHashJoin,
        Self::CrossStageSortTopK,
        Self::CrossStageSegmentedTopK,
        Self::CrossStageAggregateMinMax,
    ];

    fn query(self) -> &'static str {
        match self {
            Self::ShuffledHashJoin => {
                r#"
                SELECT f.id
                FROM df_files f
                WHERE f.id <= 15
                  AND f.id IN (
                      SELECT p.file_id FROM df_pages p WHERE p.page_text ||| 'page'
                  )
                ORDER BY f.id ASC
                LIMIT 20
                "#
            }
            Self::CrossStageSortTopK => {
                r#"
                SELECT f.score_bucket, COUNT(*), SUM(p.size_bytes)
                FROM df_files f
                JOIN df_pages p ON f.id = p.file_id
                WHERE f.content ||| 'section'
                GROUP BY f.score_bucket
                ORDER BY f.score_bucket ASC
                LIMIT 5
                "#
            }
            Self::CrossStageSegmentedTopK => {
                r#"
                SELECT f.id, f.title
                FROM df_files f
                WHERE f.content ||| 'section'
                  AND f.id IN (
                      SELECT p.file_id FROM df_pages p WHERE p.page_text ||| 'page'
                  )
                ORDER BY f.title ASC
                LIMIT 10
                "#
            }
            Self::CrossStageAggregateMinMax => {
                r#"
                SELECT MIN(f.score_bucket)
                FROM df_files f
                WHERE f.content ||| 'section'
                  AND f.id IN (
                      SELECT p.file_id FROM df_pages p WHERE p.page_text ||| 'page'
                  )
                "#
            }
        }
    }

    fn expected_producer_fragment(self) -> &'static str {
        match self {
            Self::ShuffledHashJoin => "HashJoinExec: mode=Partitioned",
            Self::CrossStageSortTopK => "SortExec: TopK(fetch=5)",
            Self::CrossStageSegmentedTopK => "SegmentedTopKExec:",
            Self::CrossStageAggregateMinMax => "AggregateExec: mode=Partial",
        }
    }

    async fn fetch_canonical_rows(self, conn: &mut PgConnection) -> Result<Vec<String>> {
        let sql = AssertSqlSafe(self.query());
        match self {
            Self::ShuffledHashJoin => {
                let rows: Vec<(i32,)> = sqlx::query_as(sql).fetch_all(conn).await?;
                Ok(rows.into_iter().map(|(id,)| format!("{id}")).collect())
            }
            Self::CrossStageSortTopK => {
                let rows: Vec<(i32, i64, i64)> = sqlx::query_as(sql).fetch_all(conn).await?;
                Ok(rows
                    .into_iter()
                    .map(|(bucket, count, sum)| format!("{bucket}:{count}:{sum}"))
                    .collect())
            }
            Self::CrossStageSegmentedTopK => {
                let rows: Vec<(i32, String)> = sqlx::query_as(sql).fetch_all(conn).await?;
                Ok(rows
                    .into_iter()
                    .map(|(id, title)| format!("{id}:{title}"))
                    .collect())
            }
            Self::CrossStageAggregateMinMax => {
                let rows: Vec<(Option<i32>,)> = sqlx::query_as(sql).fetch_all(conn).await?;
                Ok(rows
                    .into_iter()
                    .map(|(min_v,)| format!("{min_v:?}"))
                    .collect())
            }
        }
    }
}

async fn explain_analyze(conn: &mut PgConnection, query: &str) -> Result<String> {
    let rows: Vec<(String,)> = sqlx::query_as(AssertSqlSafe(format!(
        "EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, SUMMARY OFF) {query}"
    )))
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn parse_dynamic_filter_updates_received(plan: &str) -> Option<u64> {
    let prefix = "dynamic_filter_updates_received=";
    let rest = plan.split_once(prefix)?.1;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

#[rstest]
#[tokio::test]
async fn mpp_cross_stage_dynamic_filters(database: Db) -> Result<()> {
    let mut conn = database.connection().await;
    conn.execute(SETUP_SQL).await?;
    conn.execute(COMMON_GUCS).await?;

    for case in CrossStageDynamicFilterCase::ALL {
        conn.execute("SET max_parallel_workers_per_gather TO 0;")
            .await?;
        let serial_rows = case
            .fetch_canonical_rows(&mut conn)
            .await
            .with_context(|| format!("{case:?}: serial query failed"))?;
        assert!(
            !serial_rows.is_empty(),
            "{case:?}: serial baseline returned no rows"
        );

        conn.execute("SET max_parallel_workers_per_gather TO 3;")
            .await?;
        let mpp_rows = case
            .fetch_canonical_rows(&mut conn)
            .await
            .with_context(|| format!("{case:?}: MPP query failed"))?;
        assert_eq!(
            mpp_rows, serial_rows,
            "{case:?}: MPP rows diverged from serial baseline"
        );

        let plan = explain_analyze(&mut conn, case.query())
            .await
            .with_context(|| format!("{case:?}: EXPLAIN ANALYZE failed"))?;

        assert!(
            plan.contains("DistributedExec"),
            "{case:?}: expected DistributedExec in MPP plan:\n{plan}"
        );
        assert!(
            plan.contains("NetworkShuffleExec"),
            "{case:?}: expected NetworkShuffleExec stage boundary in MPP plan:\n{plan}"
        );
        let producer = case.expected_producer_fragment();
        assert!(
            plan.contains(producer),
            "{case:?}: expected producer `{producer}` in MPP plan:\n{plan}"
        );
        assert!(
            plan.contains("dynamic_filters="),
            "{case:?}: expected leaf PgSearchScan dynamic filter consumer in MPP plan:\n{plan}"
        );

        let updates_received = parse_dynamic_filter_updates_received(&plan).unwrap_or(0);
        assert!(
            updates_received > 0,
            "{case:?}: expected coordinator to receive cross-stage dynamic filter updates \
             (dynamic_filter_updates_received > 0), got {updates_received} in:\n{plan}"
        );
    }

    Ok(())
}
