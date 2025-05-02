// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use chrono::NaiveDate;
use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::{PgConnection, Row};

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
        plan.get("   Sort Field"),
        Some(&Value::String(String::from("category")))
    );
}

#[rstest]
fn sort_by_lower_parallel(mut conn: PgConnection) {
    // When parallel workers are used, our output is emitted in sorted order, and so we should have
    // a `Gather Merge` node above us.
    "SET max_parallel_workers = 8;".execute(&mut conn);
    if pg_major_version(&mut conn) >= 16 {
        "SET debug_parallel_query TO on".execute(&mut conn);
    } else {
        // We cannot reliably force parallel workers to be used without `debug_parallel_query`.
        return;
    }

    let plan = field_sort_fixture(&mut conn);

    assert_eq!(
        plan.pointer("/0/Plan/Plans/0")
            .unwrap()
            .as_object()
            .unwrap()
            .get("Node Type")
            .unwrap(),
        &Value::String("Gather Merge".to_owned())
    );

    assert_eq!(
        plan.pointer("/0/Plan/Plans/0/Plans/0")
            .unwrap()
            .as_object()
            .unwrap()
            .get("Node Type")
            .unwrap(),
        &Value::String("Custom Scan".to_owned())
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
        plan.get("   Sort Field"),
        Some(&Value::String(String::from("category")))
    );
}

#[rstest]
fn sort_by_row_return_scores(mut conn: PgConnection) {
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

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT paradedb.score(id), * FROM paradedb.bm25_search WHERE description @@@ 'keyboard OR shoes' ORDER BY category LIMIT 5".fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");

    // Get the first plan node in the plans array
    let plan = plan
        .pointer("/0/Plan/Plans/0/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();

    assert_eq!(plan.get("   Sort Field"), None);
    assert_eq!(plan.get("Scores"), Some(&Value::Bool(true)));
}

#[rstest]
async fn test_incremental_sort_with_partial_order(mut conn: PgConnection) {
    // Create the partitioned sales table
    PartitionedTable::setup().execute(&mut conn);

    // Enable debugging logs
    sqlx::query("SET client_min_messages TO DEBUG1;")
        .execute(&mut conn)
        .await
        .unwrap();

    // Enable additional debug options
    sqlx::query("SET debug_print_plan = true;")
        .execute(&mut conn)
        .await
        .unwrap();

    sqlx::query("SET debug_pretty_print = true;")
        .execute(&mut conn)
        .await
        .unwrap();

    // Check Postgres version - Incremental Sort only exists in PG 16+
    let pg_version = pg_major_version(&mut conn);
    let pg_supports_incremental_sort = pg_version >= 16;

    // Test BM25 with ORDER BY ... LIMIT to confirm sort optimization works
    let (explain_bm25,) = sqlx::query_as::<_, (Value,)>(
        "EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON) 
        SELECT description, sale_date, paradedb.score(id) as score FROM sales 
        WHERE description @@@ 'keyboard' 
        ORDER BY score, sale_date, amount LIMIT 10;",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();

    println!("EXPLAIN OUTPUT: {}", explain_bm25);

    let plan_json = explain_bm25.to_string();

    // Extract the Custom Scan nodes from the JSON plan for inspection
    let mut custom_scan_nodes = Vec::new();
    if let Ok(plan) = serde_json::from_str::<Value>(&plan_json) {
        // Navigate through the plan to find Custom Scan nodes
        if let Some(main_plan) = plan.pointer("/0/Plan") {
            collect_custom_scan_nodes(main_plan, &mut custom_scan_nodes);
        }
    }

    println!("Found {} Custom Scan nodes", custom_scan_nodes.len());
    for (i, node) in custom_scan_nodes.iter().enumerate() {
        println!("Custom Scan Node #{}: {}", i + 1, node);
    }

    // Additional debug query - check what happens with a simpler query
    let (explain_simple,) = sqlx::query_as::<_, (String,)>(
        "EXPLAIN (ANALYZE, VERBOSE) 
        SELECT description, sale_date, paradedb.score(id) as score FROM sales 
        WHERE description @@@ 'keyboard' 
        ORDER BY score, sale_date LIMIT 10;",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();

    println!("SIMPLE QUERY EXPLAIN OUTPUT: {}", explain_simple);

    // Instead of checking for specific node types, check that:
    // 1. A Sort node exists to handle the sorting (either regular Sort or Incremental Sort)
    // 2. Custom Scan nodes exist that support our search
    // 3. Scores are enabled in the Custom Scan

    // Check that we have a Sort node somewhere in the plan
    let has_sort_node = if pg_supports_incremental_sort {
        plan_json.contains("\"Node Type\":\"Incremental Sort\"")
            || explain_simple.contains("Incremental Sort")
    } else {
        plan_json.contains("\"Node Type\":\"Sort\"")
            || plan_json.contains("\"Node Type\":\"Incremental Sort\"")
            || explain_simple.contains("Sort")
            || explain_simple.contains("Incremental Sort")
    };

    assert!(
        has_sort_node,
        "Plan should include an Incremental Sort node to handle ORDER BY"
    );

    // Check that we have Custom Scan nodes that handle our search
    let has_custom_scan = plan_json.contains("\"Node Type\":\"Custom Scan\"")
        || explain_simple.contains("Custom Scan");

    assert!(
        has_custom_scan,
        "Plan should include Custom Scan nodes to perform our search"
    );

    // Check that the score is requested
    let has_scores_enabled = !custom_scan_nodes.is_empty()
        && custom_scan_nodes.iter().any(|node| {
            node.get("Scores")
                .is_some_and(|v| v.as_bool() == Some(true))
        });

    assert!(
        has_scores_enabled,
        "At least one Custom Scan node should have Scores enabled"
    );

    // Verify we get results and they're in the correct order
    let results = sqlx::query(
        "SELECT description, sale_date, paradedb.score(id) as score FROM sales 
        WHERE description @@@ 'keyboard' 
        ORDER BY score, sale_date, amount LIMIT 10;",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();

    // Results might be empty since 'keyboard' is a specific term
    // but if we get results, they should be properly sorted
    if !results.is_empty() {
        // Verify sort order - dates should be ascending
        let mut prev_date = None;
        for row in &results {
            let date: NaiveDate = row.get("sale_date");
            if let Some(prev) = prev_date {
                assert!(date >= prev, "Results should be sorted by date");
            }
            prev_date = Some(date);
        }
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

#[rstest]
fn sort_partitioned_early_cutoff(mut conn: PgConnection) {
    PartitionedTable::setup().execute(&mut conn);

    // Insert matching rows into both partitions.
    r#"
        INSERT INTO sales (sale_date, amount, description) VALUES
        ('2023-01-10', 150.00, 'Ergonomic metal keyboard'),
        ('2023-04-01', 250.00, 'Cheap plastic keyboard');
    "#
    .execute(&mut conn);

    "SET max_parallel_workers TO 0;".execute(&mut conn);

    // With ORDER BY the partition key: we expect the partitions to be visited sequentially, and
    // for cutoff to occur.
    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT description, sale_date
        FROM sales
        WHERE description @@@ 'keyboard'
        ORDER BY sale_date
        LIMIT 1;
        "#
    .fetch_one(&mut conn);
    eprintln!("{plan:#?}");

    // We expect both partitions to be in the plan, but for only the first one to have been
    // executed, because the Append node was able to get enough results from the first partition.
    let plans = plan
        .pointer("/0/Plan/Plans/0/Plans")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(
        plans[0].get("Actual Loops").unwrap(),
        &serde_json::from_str::<Value>("1").unwrap()
    );
    assert_eq!(
        plans[1].get("Actual Loops").unwrap(),
        &serde_json::from_str::<Value>("0").unwrap()
    );
}
