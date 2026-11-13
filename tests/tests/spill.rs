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

//! Parameterized integration tests for DataFusion `BufFile`-backed spilling
//! (`paradedb.spill_to_disk = on`) across serial and MPP execution and the plan
//! shapes that upstream DataFusion can spill (`AggregateExec`, `RepartitionExec`).

use rstest::*;
use sqlx::{PgConnection, Row};
use tests::fixtures::*;

#[derive(Clone, Copy, Debug)]
enum ExecutionMode {
    Serial,
    Mpp,
}

#[derive(Clone, Copy, Debug)]
enum SpillShape {
    /// Single-table DataFusion `AggregateScan` grouping on multiple column types
    /// (`TEXT`, `UUID`, `NUMERIC`, `JSONB ->>`, nullable values) with `COUNT(*)`,
    /// `SUM`, `AVG`, `MIN`, `MAX`, and `COUNT(DISTINCT)`.
    MultiType,
    /// Single-table DataFusion `AggregateScan` whose `GROUP BY` includes the
    /// primary key while projecting non-grouped `TEXT`, `NUMERIC`, and nullable
    /// `INTEGER` columns (`pdb_carry_value` and `first_value(... ORDER BY ctid)`).
    DependentRowValue,
    /// Two-table `AggregateScan` with a small `HashJoinExec` build side
    /// (`spill_categories`, 16 rows) feeding a high-cardinality `GROUP BY` over
    /// `spill_events`, spilling the aggregate / shuffle stages while the build-side
    /// hash table stays within `work_mem`.
    SmallBuildJoin,
}

impl SpillShape {
    /// Returns a `work_mem` budget below the full aggregate hash table size (so
    /// `spill_to_disk = off` fails cleanly and `spill_to_disk = on` spills), while
    /// leaving enough room for `MultiLevelMerge` to buffer at least two spilled
    /// `RecordBatch` streams per partition.
    ///
    /// Serial execution processes all ~28k groups in one `AggregateExec` pipeline,
    /// whereas 3-worker MPP partitions the 3 immutable segments evenly across three
    /// workers (~9.3k groups per worker).
    fn work_mem(self, mode: ExecutionMode) -> &'static str {
        match (mode, self) {
            (ExecutionMode::Serial, Self::MultiType) => "16MB",
            (ExecutionMode::Serial, Self::DependentRowValue) => "12MB",
            (ExecutionMode::Serial, Self::SmallBuildJoin) => "10MB",
            (ExecutionMode::Mpp, Self::MultiType) => "4MB",
            (ExecutionMode::Mpp, Self::DependentRowValue) => "4MB",
            (ExecutionMode::Mpp, Self::SmallBuildJoin) => "4MB",
        }
    }

    /// Returns `(candidate_bm25_query, baseline_pg_query)`.
    fn queries(self) -> (&'static str, &'static str) {
        match self {
            Self::MultiType => (
                r#"
                SELECT
                    group_key,
                    item_uuid,
                    metadata->>'brand' AS brand,
                    price,
                    COUNT(*) AS cnt,
                    SUM(price) AS sum_price,
                    AVG(qty) AS avg_qty,
                    MIN(qty) AS min_qty,
                    MAX(qty) AS max_qty,
                    COUNT(DISTINCT category_id) AS distinct_cats
                FROM spill_events
                WHERE content @@@ 'Section'
                GROUP BY group_key, item_uuid, metadata->>'brand', price
                "#,
                r#"
                SELECT
                    group_key,
                    item_uuid,
                    metadata->>'brand' AS brand,
                    price,
                    COUNT(*) AS cnt,
                    SUM(price) AS sum_price,
                    AVG(qty) AS avg_qty,
                    MIN(qty) AS min_qty,
                    MAX(qty) AS max_qty,
                    COUNT(DISTINCT category_id) AS distinct_cats
                FROM spill_events
                WHERE content LIKE '%Section%'
                GROUP BY group_key, item_uuid, metadata->>'brand', price
                "#,
            ),
            Self::DependentRowValue => (
                r#"
                SELECT
                    id,
                    item_uuid,
                    metadata->>'brand' AS brand,
                    group_key,
                    price,
                    qty,
                    COUNT(*) AS cnt,
                    SUM(price) AS sum_price
                FROM spill_events
                WHERE content @@@ 'Section'
                GROUP BY id, item_uuid, metadata->>'brand'
                "#,
                r#"
                SELECT
                    id,
                    item_uuid,
                    metadata->>'brand' AS brand,
                    group_key,
                    price,
                    qty,
                    COUNT(*) AS cnt,
                    SUM(price) AS sum_price
                FROM spill_events
                WHERE content LIKE '%Section%'
                GROUP BY id, item_uuid, metadata->>'brand'
                "#,
            ),
            Self::SmallBuildJoin => (
                r#"
                SELECT
                    e.group_key,
                    e.item_uuid,
                    c.label,
                    COUNT(*) AS cnt,
                    SUM(e.price) AS sum_price,
                    AVG(e.qty) AS avg_qty
                FROM spill_events e
                JOIN spill_categories c ON e.category_id = c.id
                WHERE e.content @@@ 'Section'
                GROUP BY e.group_key, e.item_uuid, c.label
                "#,
                r#"
                SELECT
                    e.group_key,
                    e.item_uuid,
                    c.label,
                    COUNT(*) AS cnt,
                    SUM(e.price) AS sum_price,
                    AVG(e.qty) AS avg_qty
                FROM spill_events e
                JOIN spill_categories c ON e.category_id = c.id
                WHERE e.content LIKE '%Section%'
                GROUP BY e.group_key, e.item_uuid, c.label
                "#,
            ),
        }
    }
}

fn setup_spill_tables(conn: &mut PgConnection) {
    let mut inserts = String::new();
    for seg in 0..3 {
        let start = seg * 10_000 + 1;
        let end = (seg + 1) * 10_000;
        inserts.push_str(&format!(
            r#"
            INSERT INTO spill_events (
                id, category_id, item_uuid, group_key, content, price, qty, metadata
            )
            SELECT
                g,
                ((g - 1) % 16) + 1,
                rpad(lpad(g::text, 10, '0'), 32, 'a')::uuid,
                'group-key-payload-' || g,
                CASE WHEN g % 29 = 0 THEN 'Ignored ' || g ELSE 'Section ' || g END,
                (((g * 13) % 100000) / 100.0)::numeric(10, 2),
                CASE WHEN g % 10 = 0 THEN NULL ELSE (g * 17) % 4096 END,
                CASE
                    WHEN g % 7 = 0 THEN NULL
                    ELSE jsonb_build_object(
                        'brand',
                        (ARRAY['acme', 'globex', 'initech', 'umbrella'])[((g - 1) % 4) + 1]
                    )
                END
            FROM generate_series({start}, {end}) AS g;
            "#
        ));
    }

    format!(
        r#"
    CREATE TABLE spill_categories (
        id INTEGER PRIMARY KEY,
        label TEXT NOT NULL
    );

    CREATE TABLE spill_events (
        id INTEGER PRIMARY KEY,
        category_id INTEGER NOT NULL,
        item_uuid UUID NOT NULL,
        group_key TEXT NOT NULL,
        content TEXT NOT NULL,
        price NUMERIC(10, 2) NOT NULL,
        qty INTEGER,
        metadata JSONB
    );

    ALTER TABLE spill_categories SET (autovacuum_enabled = off);
    ALTER TABLE spill_events SET (autovacuum_enabled = off);

    CREATE INDEX spill_categories_idx ON spill_categories
    USING paradedb (id, (label::pdb.literal))
    WITH (
        target_segment_count = 3,
        layer_sizes = '0',
        background_layer_sizes = '0'
    );

    CREATE INDEX spill_events_idx ON spill_events
    USING paradedb (
        id,
        category_id,
        item_uuid,
        (group_key::pdb.literal),
        content,
        price,
        qty,
        (metadata::pdb.unicode_words('columnar=true'))
    )
    WITH (
        target_segment_count = 3,
        layer_sizes = '0',
        background_layer_sizes = '0'
    );

    SET paradedb.global_mutable_segment_rows = 0;

    INSERT INTO spill_categories (id, label)
    SELECT g, 'cat-' || g FROM generate_series(1, 5) AS g;
    INSERT INTO spill_categories (id, label)
    SELECT g, 'cat-' || g FROM generate_series(6, 10) AS g;
    INSERT INTO spill_categories (id, label)
    SELECT g, 'cat-' || g FROM generate_series(11, 16) AS g;

    {inserts}

    -- Exercise MVCC visibility filtering across both deleted immutable-segment rows
    -- and updated rows residing in a mutable segment.
    SET paradedb.global_mutable_segment_rows = 1000;
    DELETE FROM spill_events WHERE id % 31 = 0;
    UPDATE spill_events
    SET qty = COALESCE(qty, 0) + 5,
        price = price + 1.25,
        content = 'Updated Section ' || id
    WHERE id % 97 = 0 AND id % 31 <> 0;
    RESET paradedb.global_mutable_segment_rows;

    ANALYZE spill_categories;
    ANALYZE spill_events;
    "#
    )
    .execute(conn);
}

fn configure_candidate_gucs(conn: &mut PgConnection, mode: ExecutionMode, shape: SpillShape) {
    let parallelism_sql = match mode {
        ExecutionMode::Serial => "SET max_parallel_workers_per_gather TO 0;",
        ExecutionMode::Mpp => {
            r#"
            SET paradedb.mpp_min_rows TO 0;
            SET paradedb.min_rows_per_worker TO 10;
            SET max_parallel_workers_per_gather TO 3;
            SET max_parallel_workers TO 64;
            SET min_parallel_table_scan_size TO 0;
            SET parallel_setup_cost TO 0;
            SET parallel_tuple_cost TO 0;
            "#
        }
    };

    format!(
        r#"
        SET paradedb.enable_custom_scan TO on;
        SET paradedb.enable_aggregate_custom_scan TO on;
        SET paradedb.enable_join_custom_scan TO on;
        SET paradedb.max_term_agg_buckets TO 1;
        SET paradedb.enable_columnar_exec TO true;
        SET enable_indexscan TO off;
        SET hash_mem_multiplier TO 1.0;
        SET work_mem TO '{work_mem}';
        {parallelism_sql}
        "#,
        work_mem = shape.work_mem(mode),
    )
    .execute(conn);
}

/// Checks whether `EXPLAIN (ANALYZE, VERBOSE)` output contains at least one
/// non-zero `spill_count`, handling both scalar (`spill_count=2`) and MPP
/// per-task map (`spill_count={0: 0, 1: 2}`) renderings.
fn has_nonzero_spill_count(plan: &str) -> bool {
    let mut rest = plan;
    while let Some(pos) = rest.find("spill_count=") {
        rest = &rest[pos + "spill_count=".len()..];
        if let Some(after_brace) = rest.strip_prefix('{') {
            if let Some(end) = after_brace.find('}') {
                let entries = &after_brace[..end];
                if entries.split(',').any(|entry| {
                    entry
                        .split_once(':')
                        .and_then(|(_, val)| val.trim().parse::<usize>().ok())
                        .is_some_and(|n| n > 0)
                }) {
                    return true;
                }
            }
        } else {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.parse::<usize>().is_ok_and(|n| n > 0) {
                return true;
            }
        }
    }
    false
}

fn fetch_sorted_rows(conn: &mut PgConnection, query: &str) -> Vec<String> {
    let rows = query.fetch_dynamic(conn);
    let mut formatted: Vec<String> = rows
        .into_iter()
        .map(|row| {
            let mut parts = Vec::with_capacity(row.len());
            for i in 0..row.len() {
                let cell = if let Ok(val) = row.try_get::<i64, _>(i) {
                    val.to_string()
                } else if let Ok(val) = row.try_get::<i32, _>(i) {
                    val.to_string()
                } else if let Ok(val) = row.try_get::<sqlx::types::BigDecimal, _>(i) {
                    format!("{val:.6}")
                } else if let Ok(val) = row.try_get::<f64, _>(i) {
                    format!("{val:.6}")
                } else if let Ok(val) = row.try_get::<sqlx::types::Uuid, _>(i) {
                    val.to_string()
                } else if let Ok(val) = row.try_get::<String, _>(i) {
                    val
                } else {
                    "NULL".to_string()
                };
                parts.push(cell);
            }
            parts.join("|")
        })
        .collect();
    formatted.sort();
    formatted
}

#[rstest]
#[case::multi_type_group_by(SpillShape::MultiType)]
#[case::dependent_row_value_group_by(SpillShape::DependentRowValue)]
#[case::small_build_join_group_by(SpillShape::SmallBuildJoin)]
fn datafusion_spill_to_disk(
    // Run cases serially so concurrent MPP worker pools and `CREATE DATABASE` / `DROP DATABASE`
    // churn do not perturb Stage 2 partition overlap near tight `work_mem` thresholds.
    #[from(serial_conn)] mut conn: SerialConn,
    #[case] shape: SpillShape,
    #[values(ExecutionMode::Serial, ExecutionMode::Mpp)] mode: ExecutionMode,
) {
    setup_spill_tables(&mut conn);
    configure_candidate_gucs(&mut conn, mode, shape);

    let (bm25_query, pg_query) = shape.queries();

    // 1. With `paradedb.spill_to_disk = off` (default), exceeding `work_mem` must fail cleanly.
    "SET paradedb.spill_to_disk TO off;".execute(&mut conn);
    let err = bm25_query
        .fetch_dynamic_result(&mut conn)
        .expect_err("expected query to fail on work_mem overflow with spill_to_disk=off");
    let err_text = err.to_string();
    assert!(
        err_text.contains("raise work_mem")
            || err_text.contains("DiskManager")
            || err_text.contains("temporary files are not enabled")
            || err_text.contains("transport receiver detached"),
        "expected work_mem / disabled DiskManager error for {mode:?} {shape:?}, got: {err_text}"
    );

    // 2. With `paradedb.spill_to_disk = on`, `EXPLAIN (ANALYZE, VERBOSE)` must confirm the
    // expected plan shape and a non-zero `spill_count`.
    "SET paradedb.spill_to_disk TO on;".execute(&mut conn);
    let explain_lines: Vec<String> =
        format!("EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, BUFFERS OFF) {bm25_query}")
            .fetch_scalar(&mut conn);
    let plan_text = explain_lines.join("\n");

    assert!(
        plan_text.contains("ParadeDB Aggregate Scan")
            && plan_text.contains("DataFusion Physical Plan"),
        "expected DataFusion ParadeDB Aggregate Scan for {mode:?} {shape:?}, got:\n{plan_text}"
    );
    match mode {
        ExecutionMode::Serial => assert!(
            !plan_text.contains("DistributedExec"),
            "did not expect DistributedExec in serial plan:\n{plan_text}"
        ),
        ExecutionMode::Mpp => assert!(
            plan_text.contains("DistributedExec"),
            "expected DistributedExec in MPP plan:\n{plan_text}"
        ),
    }
    if matches!(shape, SpillShape::SmallBuildJoin) {
        assert!(
            plan_text.contains("HashJoinExec"),
            "expected HashJoinExec in join aggregate plan:\n{plan_text}"
        );
    }
    assert!(
        has_nonzero_spill_count(&plan_text),
        "expected non-zero spill_count for {mode:?} {shape:?}, got:\n{plan_text}"
    );

    // 3. Verify exact result parity against native PostgreSQL execution.
    let candidate_rows = fetch_sorted_rows(&mut conn, bm25_query);
    assert!(
        candidate_rows.len() > 25_000,
        "expected >25k groups after filtering and MVCC churn, got {}",
        candidate_rows.len()
    );

    r#"
    SET paradedb.enable_custom_scan TO off;
    SET paradedb.enable_aggregate_custom_scan TO off;
    SET paradedb.enable_join_custom_scan TO off;
    SET work_mem TO '256MB';
    "#
    .execute(&mut conn);
    let baseline_rows = fetch_sorted_rows(&mut conn, pg_query);

    assert_eq!(
        candidate_rows, baseline_rows,
        "spilled custom scan results differed from PostgreSQL for {mode:?} {shape:?}"
    );
}

#[test]
fn parses_scalar_and_mpp_spill_counts() {
    assert!(!has_nonzero_spill_count(
        "AggregateExec: mode=Single, metrics=[spill_count=0]"
    ));
    assert!(has_nonzero_spill_count(
        "AggregateExec: mode=Single, metrics=[spill_count=3]"
    ));
    assert!(!has_nonzero_spill_count(
        "AggregateExec: mode=FinalPartitioned, metrics=[spill_count={0: 0, 1: 0, 2: 0}]"
    ));
    assert!(has_nonzero_spill_count(
        "AggregateExec: mode=FinalPartitioned, metrics=[spill_count={0: 0, 1: 2, 2: 0}]"
    ));
}
