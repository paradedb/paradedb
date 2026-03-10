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
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

/// Collects partition-level "Actual Loops" counts from all Custom Scan nodes in the plan tree.
/// In parallel plans, PG may emit per-loop averages as floats (e.g. 1.0), so we accept both.
fn collect_actual_loops(plan: &Value, loops: &mut Vec<i64>) {
    if let Some(node_type) = plan.get("Node Type").and_then(|v| v.as_str()) {
        if node_type == "Custom Scan" {
            if let Some(al) = plan
                .get("Actual Loops")
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            {
                loops.push(al);
            }
        }
    }
    if let Some(plans) = plan.get("Plans").and_then(|p| p.as_array()) {
        for child in plans {
            collect_actual_loops(child, loops);
        }
    }
}

/// Collects node type names from plan tree in pre-order.
fn collect_node_types(plan: &Value, types: &mut Vec<String>) {
    if let Some(node_type) = plan.get("Node Type").and_then(|v| v.as_str()) {
        types.push(node_type.to_string());
    }
    if let Some(plans) = plan.get("Plans").and_then(|p| p.as_array()) {
        for child in plans {
            collect_node_types(child, types);
        }
    }
}

/// Collects partition-level "Actual Rows" counts from all Custom Scan nodes in the plan tree.
/// In parallel plans, PG may emit per-loop averages as floats (e.g. 5.0), so we accept both.
fn collect_actual_rows(plan: &Value, rows: &mut Vec<i64>) {
    if let Some(node_type) = plan.get("Node Type").and_then(|v| v.as_str()) {
        if node_type == "Custom Scan" {
            if let Some(ar) = plan
                .get("Actual Rows")
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            {
                rows.push(ar);
            }
        }
    }
    if let Some(plans) = plan.get("Plans").and_then(|p| p.as_array()) {
        for child in plans {
            collect_actual_rows(child, rows);
        }
    }
}

/// Collects exec method strings from Custom Scan nodes.
fn collect_exec_methods(plan: &Value, methods: &mut Vec<String>) {
    if let Some(node_type) = plan.get("Node Type").and_then(|v| v.as_str()) {
        if node_type == "Custom Scan" {
            if let Some(method) = plan.get("Exec Method").and_then(|v| v.as_str()) {
                methods.push(method.to_string());
            }
        }
    }
    if let Some(plans) = plan.get("Plans").and_then(|p| p.as_array()) {
        for child in plans {
            collect_exec_methods(child, methods);
        }
    }
}

fn field_sort_fixture(conn: &mut PgConnection) -> Value {
    // ensure our custom scan wins against our small test table
    r#"
        SET enable_indexscan TO off;
        CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

        CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
        WITH (
            key_field = 'id',
            text_fields = '{
                "description": {},
                "category": {
                    "tokenizer": {"type": "keyword"},
                    "fast": true,
                    "normalizer": "lowercase"
                }
            }',
            numeric_fields = '{
                "rating": {}
            }',
            boolean_fields = '{
                "in_stock": {}
            }',
            json_fields = '{
                "metadata": {}
            }',
            datetime_fields = '{
                "created_at": {},
                "last_updated_date": {},
                "latest_available_time": {}
            }'
        );
    "#.execute(conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE description @@@ 'keyboard OR shoes' ORDER BY lower(category) LIMIT 5".fetch_one::<(Value,)>(conn);
    eprintln!("{plan:#?}");
    plan
}

#[rstest]
fn sort_by_lower(mut conn: PgConnection) {
    let plan = field_sort_fixture(&mut conn);
    let plan = plan
        .pointer("/0/Plan/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        plan.get("   TopK Order By"),
        Some(&Value::String(String::from("category asc")))
    );
}

#[rstest]
fn sort_by_lower_parallel(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 17 {
        // We cannot reliably force parallel workers to be used without `debug_parallel_query`.
        return;
    }

    "SET max_parallel_workers = 8;".execute(&mut conn);
    "SET debug_parallel_query TO on".execute(&mut conn);

    let plan = field_sort_fixture(&mut conn);
    let plan = plan
        .pointer("/0/Plan/Plans/0/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        plan.get("   TopK Order By"),
        Some(&Value::String(String::from("category asc")))
    );
}

#[rstest]
fn sort_by_raw(mut conn: PgConnection) {
    // ensure our custom scan wins against our small test table
    r#"
        SET enable_indexscan TO off;
        CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

        CREATE INDEX bm25_search_idx ON paradedb.bm25_search
        USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
        WITH (
            key_field = 'id',
            text_fields = '{
                "description": {},
                "category": {
                    "tokenizer": {"type": "keyword"},
                    "fast": true,
                    "normalizer": "raw"
                }
            }',
            numeric_fields = '{
                "rating": {}
            }',
            boolean_fields = '{
                "in_stock": {}
            }',
            json_fields = '{
                "metadata": {}
            }',
            datetime_fields = '{
                "created_at": {},
                "last_updated_date": {},
                "latest_available_time": {}
            }'
        );
    "#.execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE description @@@ 'keyboard OR shoes' ORDER BY category LIMIT 5".fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");
    let plan = plan
        .pointer("/0/Plan/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(
        plan.get("   TopK Order By"),
        Some(&Value::String(String::from("category asc")))
    );
}

#[rstest]
async fn test_compound_sort(mut conn: PgConnection) {
    "SET max_parallel_workers to 0;".execute(&mut conn);

    SimpleProductsTable::setup().execute(&mut conn);

    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id FROM paradedb.bm25_search
        WHERE description @@@ 'shoes' ORDER BY rating DESC, created_at DESC LIMIT 10"#
        .fetch_one(&mut conn);

    eprintln!("plan: {plan:#?}");

    // Since both ORDER-BY fields are fast, they should be pushed down.
    assert_eq!(
        plan.pointer("/0/Plan/Plans/0/   TopK Order By"),
        Some(&Value::String(String::from("rating desc, created_at desc")))
    );
}

#[rstest]
async fn compound_sort_expression(mut conn: PgConnection) {
    "SET max_parallel_workers to 0;".execute(&mut conn);

    SimpleProductsTable::setup().execute(&mut conn);

    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT *, pdb.score(id) * 2 FROM paradedb.bm25_search
        WHERE description @@@ 'shoes' ORDER BY 2, pdb.score(id) LIMIT 10"#
        .fetch_one(&mut conn);

    eprintln!("plan: {plan:#?}");

    // Since the ORDER BY contains an expression, we should not attempt Top K, even if other
    // fields could be pushed down.
    assert_eq!(
        plan.pointer("/0/Plan/Plans/0/Plans/0/Exec Method"),
        Some(&Value::String(String::from("NormalScanExecState")))
    );
}

#[rstest]
async fn compound_sort_partitioned(mut conn: PgConnection) {
    "SET max_parallel_workers to 0;".execute(&mut conn);

    // Create the partitioned sales table
    PartitionedTable::setup().execute(&mut conn);

    // Insert a good size amount of random data, and then analyze.
    r#"
    INSERT INTO sales (sale_date, amount, description)
    SELECT
        (DATE '2023-01-01' + (random() * 179)::integer) AS sale_date,
        (random() * 1000)::real AS amount,
        ('wine '::text || md5(random()::text)) AS description
    FROM generate_series(1, 1000);

    ANALYZE;
    "#
    .execute(&mut conn);

    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id, sale_date, amount FROM sales
        WHERE description @@@ 'wine'
        ORDER BY sale_date, amount LIMIT 10;"#
        .fetch_one(&mut conn);

    eprintln!("plan: {plan:#?}");

    // Extract the Custom Scan nodes from the JSON plan for inspection
    let mut custom_scan_nodes = Vec::new();
    collect_custom_scan_nodes(plan.pointer("/0/Plan").unwrap(), &mut custom_scan_nodes);

    // Check that we have Custom Scan nodes that handle our search
    assert_eq!(custom_scan_nodes.len(), 2);
    for node in custom_scan_nodes {
        assert_eq!(
            node.get("   TopK Order By"),
            Some(&Value::String(String::from("sale_date asc, amount asc")))
        );
    }
}

// Helper function to recursively collect Custom Scan nodes from a plan
fn collect_custom_scan_nodes(plan: &Value, nodes: &mut Vec<Value>) {
    // Check if this is a Custom Scan node
    if let Some(node_type) = plan.get("Node Type").and_then(|v| v.as_str()) {
        if node_type == "Custom Scan" {
            nodes.push(plan.clone());
        }
    }

    // Recursively check child plans
    if let Some(plans) = plan.get("Plans").and_then(|p| p.as_array()) {
        for child_plan in plans {
            collect_custom_scan_nodes(child_plan, nodes);
        }
    }
}

/// Non-parallel baseline — verify early partition cutoff with LIMIT on
/// a large partitioned table. With parallelism disabled and ORDER BY the partition
/// key, later partitions should show Actual Loops = 0 (never executed).
#[rstest]
fn parallel_topk_partition_baseline_no_parallel(mut conn: PgConnection) {
    LargePartitionedTable::setup().execute(&mut conn);

    "SET max_parallel_workers_per_gather = 0;".execute(&mut conn);
    "SET enable_indexscan TO off;".execute(&mut conn);

    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id, sale_date, amount FROM sales_large
        WHERE description @@@ 'wine'
        ORDER BY sale_date
        LIMIT 5;
    "#
    .fetch_one(&mut conn);
    eprintln!("=== NON-PARALLEL BASELINE (ASC) ===");
    eprintln!("{plan:#?}");

    let root = plan.pointer("/0/Plan").unwrap();

    // Collect node types to verify plan shape (should be Merge Append, not Gather Merge)
    let mut node_types = Vec::new();
    collect_node_types(root, &mut node_types);
    eprintln!("Node types: {node_types:?}");

    // We expect Limit -> Append (or Merge Append) -> Custom Scan... pattern.
    // ParadeDB's Custom Scan handles sorting internally, so PG may use a plain Append.
    assert!(
        node_types.contains(&"Append".to_string())
            || node_types.contains(&"Merge Append".to_string()),
        "Non-parallel plan should use Append or Merge Append, got: {node_types:?}"
    );
    assert!(
        !node_types.contains(&"Gather Merge".to_string())
            && !node_types.contains(&"Gather".to_string()),
        "Non-parallel plan should NOT use Gather/Gather Merge, got: {node_types:?}"
    );

    // Collect actual loops from Custom Scan nodes
    let mut loops = Vec::new();
    collect_actual_loops(root, &mut loops);
    eprintln!("Actual loops per partition: {loops:?}");

    // With LIMIT 5 and ORDER BY sale_date ASC, only the earliest partition(s) should execute.
    // Later partitions should have Actual Loops = 0.
    assert!(
        loops.len() == 8,
        "Expected 8 partition scans, got {}",
        loops.len()
    );
    let executed_count = loops.iter().filter(|&&l| l > 0).count();
    let skipped_count = loops.iter().filter(|&&l| l == 0).count();
    eprintln!("Partitions executed: {executed_count}, skipped: {skipped_count}");
    assert!(
        skipped_count > 0,
        "Expected at least some partitions to be skipped (Actual Loops = 0), but all were executed: {loops:?}"
    );

    // Verify exec methods are TopK
    let mut methods = Vec::new();
    collect_exec_methods(root, &mut methods);
    eprintln!("Exec methods: {methods:?}");
}

/// Verify that a parallel TopK query over a partitioned table:
/// 1. Uses a parallel plan (Gather Merge or Gather + Append)
/// 2. Terminates early — later partitions produce 0 rows via cross-partition
///    early termination (they still show Actual Loops = 1 because each worker
///    enters the partition, but the early term check causes immediate return).
///
/// With `ORDER BY sale_date ASC LIMIT 5`, only the earliest partition(s) should
/// produce rows; later partitions should produce 0 rows.
#[rstest]
fn parallel_topk_partition_early_term_asc(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 17 {
        return;
    }

    LargePartitionedTable::setup().execute(&mut conn);

    "SET max_parallel_workers_per_gather = 4;".execute(&mut conn);
    "SET max_parallel_workers = 8;".execute(&mut conn);
    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET debug_parallel_query TO on;".execute(&mut conn);
    // Reduce parallel costs to encourage parallel plans.
    "SET parallel_tuple_cost = 0;".execute(&mut conn);
    "SET parallel_setup_cost = 0;".execute(&mut conn);

    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id, sale_date, amount FROM sales_large
        WHERE description @@@ 'wine'
        ORDER BY sale_date
        LIMIT 5;
    "#
    .fetch_one(&mut conn);
    eprintln!("=== PARALLEL EARLY TERM (ASC) ===");
    eprintln!("{plan:#?}");

    let root = plan.pointer("/0/Plan").unwrap();

    // 1) Check the plan is parallelized
    let mut node_types = Vec::new();
    collect_node_types(root, &mut node_types);
    eprintln!("Node types: {node_types:?}");

    let has_gather = node_types.iter().any(|t| t.contains("Gather"));
    assert!(
        has_gather,
        "Expected a parallel plan with Gather/Gather Merge, got: {node_types:?}"
    );

    // 2) Check that early termination caused most partitions to produce 0 rows.
    // In a Gather Merge plan, every worker enters every partition (Actual Loops = 1),
    // but early termination makes later partitions return immediately with 0 rows.
    let mut rows_per_partition = Vec::new();
    collect_actual_rows(root, &mut rows_per_partition);
    eprintln!("Actual rows per partition: {rows_per_partition:?}");

    let producing_count = rows_per_partition.iter().filter(|&&r| r > 0).count();
    let empty_count = rows_per_partition.iter().filter(|&&r| r == 0).count();
    eprintln!("Partitions producing rows: {producing_count}, empty: {empty_count}");

    // With 8 partitions and LIMIT 5, the first partition has ~12500 matching rows.
    // Early termination should cause at least some later partitions to produce 0 rows.
    assert!(
        empty_count > 0,
        "Expected cross-partition early termination to cause some partitions to produce 0 rows, \
         but all {producing_count} partitions produced rows. Rows: {rows_per_partition:?}"
    );

    // Verify results are correct
    let rows: Vec<(i32, chrono::NaiveDate, f32)> = r#"
        SELECT id, sale_date, amount FROM sales_large
        WHERE description @@@ 'wine'
        ORDER BY sale_date
        LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5, "Expected 5 result rows");
    // Verify results are in ascending order
    for w in rows.windows(2) {
        assert!(
            w[0].1 <= w[1].1,
            "Results not in ASC order: {:?} > {:?}",
            w[0].1,
            w[1].1
        );
    }
}

/// Verify that cross-partition early termination works with DESC ordering.
/// With `ORDER BY sale_date DESC LIMIT 5`, only the latest partition(s)
/// should produce rows; earlier partitions should produce 0 rows.
#[rstest]
fn parallel_topk_partition_early_term_desc(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 17 {
        return;
    }

    LargePartitionedTable::setup().execute(&mut conn);

    "SET max_parallel_workers_per_gather = 4;".execute(&mut conn);
    "SET max_parallel_workers = 8;".execute(&mut conn);
    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET debug_parallel_query TO on;".execute(&mut conn);
    "SET parallel_tuple_cost = 0;".execute(&mut conn);
    "SET parallel_setup_cost = 0;".execute(&mut conn);

    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id, sale_date, amount FROM sales_large
        WHERE description @@@ 'wine'
        ORDER BY sale_date DESC
        LIMIT 5;
    "#
    .fetch_one(&mut conn);
    eprintln!("=== PARALLEL EARLY TERM (DESC) ===");
    eprintln!("{plan:#?}");

    let root = plan.pointer("/0/Plan").unwrap();

    // 1) Check the plan is parallelized
    let mut node_types = Vec::new();
    collect_node_types(root, &mut node_types);
    eprintln!("Node types: {node_types:?}");

    let has_gather = node_types.iter().any(|t| t.contains("Gather"));
    assert!(
        has_gather,
        "Expected a parallel plan with Gather/Gather Merge, got: {node_types:?}"
    );

    // 2) Check that early termination caused most partitions to produce 0 rows
    let mut rows_per_partition = Vec::new();
    collect_actual_rows(root, &mut rows_per_partition);
    eprintln!("Actual rows per partition: {rows_per_partition:?}");

    let empty_count = rows_per_partition.iter().filter(|&&r| r == 0).count();
    eprintln!(
        "DESC — Partitions producing rows: {}, empty: {empty_count}",
        rows_per_partition.iter().filter(|&&r| r > 0).count()
    );

    assert!(
        empty_count > 0,
        "Expected cross-partition early termination (DESC) to cause some partitions \
         to produce 0 rows, but all produced rows. Rows: {rows_per_partition:?}"
    );

    // Verify results are correct and in DESC order
    let rows: Vec<(i32, chrono::NaiveDate, f32)> = r#"
        SELECT id, sale_date, amount FROM sales_large
        WHERE description @@@ 'wine'
        ORDER BY sale_date DESC
        LIMIT 5;
    "#
    .fetch(&mut conn);
    assert_eq!(rows.len(), 5, "Expected 5 result rows");
    for w in rows.windows(2) {
        assert!(
            w[0].1 >= w[1].1,
            "Results not in DESC order: {:?} < {:?}",
            w[0].1,
            w[1].1
        );
    }
}

/// Worst-case test: matches exist ONLY in the last partition (for ASC ordering).
/// All partitions must scan because earlier partitions produce 0 results.
/// This verifies that early termination doesn't incorrectly skip the partition
/// that actually has matching data.
#[rstest]
fn parallel_topk_partition_early_term_worst_case(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 17 {
        return;
    }

    // Build a table where 'unicornterm' only appears in the last partition (2023 Q4).
    r#"
    BEGIN;
        CREATE TABLE sales_et_worst (
            id SERIAL,
            sale_date DATE NOT NULL,
            amount REAL NOT NULL,
            description TEXT,
            PRIMARY KEY (id, sale_date)
        ) PARTITION BY RANGE (sale_date);

        CREATE TABLE sales_et_worst_2023_q1 PARTITION OF sales_et_worst
          FOR VALUES FROM ('2023-01-01') TO ('2023-04-01');
        CREATE TABLE sales_et_worst_2023_q2 PARTITION OF sales_et_worst
          FOR VALUES FROM ('2023-04-01') TO ('2023-07-01');
        CREATE TABLE sales_et_worst_2023_q3 PARTITION OF sales_et_worst
          FOR VALUES FROM ('2023-07-01') TO ('2023-10-01');
        CREATE TABLE sales_et_worst_2023_q4 PARTITION OF sales_et_worst
          FOR VALUES FROM ('2023-10-01') TO ('2024-01-01');

        CREATE INDEX sales_et_worst_idx ON sales_et_worst
          USING bm25 (id, description, sale_date, amount)
          WITH (
            key_field='id',
            numeric_fields='{"amount": {"fast": true}}',
            datetime_fields='{"sale_date": {"fast": true}}'
          );

        -- Filler rows across all partitions (not matching search term)
        INSERT INTO sales_et_worst (sale_date, amount, description)
        SELECT
            (DATE '2023-01-01' + (random() * 364)::integer) AS sale_date,
            (random() * 1000)::real AS amount,
            ('filler '::text || md5(random()::text)) AS description
        FROM generate_series(1, 40000);

        -- 'unicornterm' ONLY in the last partition (2023 Q4)
        INSERT INTO sales_et_worst (sale_date, amount, description)
        SELECT
            (DATE '2023-10-01' + (random() * 91)::integer) AS sale_date,
            (random() * 1000)::real AS amount,
            ('unicornterm '::text || md5(random()::text)) AS description
        FROM generate_series(1, 1000);

        ANALYZE sales_et_worst;
    COMMIT;
    "#
    .execute(&mut conn);

    "SET max_parallel_workers_per_gather = 4;".execute(&mut conn);
    "SET max_parallel_workers = 8;".execute(&mut conn);
    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET debug_parallel_query TO on;".execute(&mut conn);
    "SET parallel_tuple_cost = 0;".execute(&mut conn);
    "SET parallel_setup_cost = 0;".execute(&mut conn);

    // All partitions must scan because earlier partitions produce 0 results,
    // so the early termination condition (sum of earlier results >= limit)
    // is never satisfied until the last partition runs.
    let rows: Vec<(i32, chrono::NaiveDate, f32)> = r#"
        SELECT id, sale_date, amount FROM sales_et_worst
        WHERE description @@@ 'unicornterm'
        ORDER BY sale_date
        LIMIT 5;
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 5, "Expected 5 results from last partition");
    // All results should be from Q4 2023 (the only partition with matches)
    for (_, date, _) in &rows {
        assert!(
            *date >= chrono::NaiveDate::from_ymd_opt(2023, 10, 1).unwrap(),
            "Expected all results from 2023 Q4, got date: {date}"
        );
    }
    // Verify ASC ordering
    for w in rows.windows(2) {
        assert!(
            w[0].1 <= w[1].1,
            "Results not in ASC order: {:?} > {:?}",
            w[0].1,
            w[1].1
        );
    }
}

/// Verify that nested (multi-level) partitions disable early termination.
/// A root table partitioned by year, with each year sub-partitioned by month,
/// should still return correct results without early termination.
#[rstest]
fn nested_partition_disables_early_termination(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 17 {
        return;
    }

    r#"
    BEGIN;
        CREATE TABLE sales_nested (
            id SERIAL,
            sale_date DATE NOT NULL,
            amount REAL NOT NULL,
            description TEXT,
            PRIMARY KEY (id, sale_date)
        ) PARTITION BY RANGE (sale_date);

        -- Year-level partitions, themselves sub-partitioned by quarter
        CREATE TABLE sales_nested_2022 PARTITION OF sales_nested
          FOR VALUES FROM ('2022-01-01') TO ('2023-01-01')
          PARTITION BY RANGE (sale_date);

        CREATE TABLE sales_nested_2023 PARTITION OF sales_nested
          FOR VALUES FROM ('2023-01-01') TO ('2024-01-01')
          PARTITION BY RANGE (sale_date);

        -- Leaf partitions for 2022
        CREATE TABLE sales_nested_2022_q1 PARTITION OF sales_nested_2022
          FOR VALUES FROM ('2022-01-01') TO ('2022-04-01');
        CREATE TABLE sales_nested_2022_q2 PARTITION OF sales_nested_2022
          FOR VALUES FROM ('2022-04-01') TO ('2022-07-01');
        CREATE TABLE sales_nested_2022_q3 PARTITION OF sales_nested_2022
          FOR VALUES FROM ('2022-07-01') TO ('2022-10-01');
        CREATE TABLE sales_nested_2022_q4 PARTITION OF sales_nested_2022
          FOR VALUES FROM ('2022-10-01') TO ('2023-01-01');

        -- Leaf partitions for 2023
        CREATE TABLE sales_nested_2023_q1 PARTITION OF sales_nested_2023
          FOR VALUES FROM ('2023-01-01') TO ('2023-04-01');
        CREATE TABLE sales_nested_2023_q2 PARTITION OF sales_nested_2023
          FOR VALUES FROM ('2023-04-01') TO ('2023-07-01');
        CREATE TABLE sales_nested_2023_q3 PARTITION OF sales_nested_2023
          FOR VALUES FROM ('2023-07-01') TO ('2023-10-01');
        CREATE TABLE sales_nested_2023_q4 PARTITION OF sales_nested_2023
          FOR VALUES FROM ('2023-10-01') TO ('2024-01-01');

        CREATE INDEX sales_nested_idx ON sales_nested
          USING bm25 (id, description, sale_date, amount)
          WITH (
            key_field='id',
            numeric_fields='{"amount": {"fast": true}}',
            datetime_fields='{"sale_date": {"fast": true}}'
          );

        INSERT INTO sales_nested (sale_date, amount, description)
        SELECT
            (DATE '2022-01-01' + (random() * 729)::integer) AS sale_date,
            (random() * 1000)::real AS amount,
            ('wine '::text || md5(random()::text)) AS description
        FROM generate_series(1, 40000);

        ANALYZE sales_nested;
    COMMIT;
    "#
    .execute(&mut conn);

    "SET max_parallel_workers_per_gather = 4;".execute(&mut conn);
    "SET max_parallel_workers = 8;".execute(&mut conn);
    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET parallel_tuple_cost = 0;".execute(&mut conn);
    "SET parallel_setup_cost = 0;".execute(&mut conn);

    // Verify correct results with ORDER BY partition key and LIMIT
    let rows: Vec<(i32, chrono::NaiveDate, f32)> = r#"
        SELECT id, sale_date, amount FROM sales_nested
        WHERE description @@@ 'wine'
        ORDER BY sale_date
        LIMIT 5;
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 5, "Expected 5 result rows");

    // Verify results are in ascending order
    for w in rows.windows(2) {
        assert!(
            w[0].1 <= w[1].1,
            "Results not in ASC order: {:?} > {:?}",
            w[0].1,
            w[1].1
        );
    }

    // Verify that early termination is NOT used (no "Terminated Early: true" in plan)
    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id, sale_date, amount FROM sales_nested
        WHERE description @@@ 'wine'
        ORDER BY sale_date
        LIMIT 5;
    "#
    .fetch_one(&mut conn);

    let root = plan.pointer("/0/Plan").unwrap();
    let mut custom_scan_nodes = Vec::new();
    collect_custom_scan_nodes(root, &mut custom_scan_nodes);

    for node in &custom_scan_nodes {
        let terminated_early = node
            .get("Terminated Early")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(
            !terminated_early,
            "Expected no early termination for nested partitions, but found \
             'Terminated Early: true' in plan: {node:#?}"
        );
    }
}
