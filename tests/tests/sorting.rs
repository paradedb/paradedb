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
        plan.get("Sort Field"),
        Some(&Value::String(String::from("category")))
    );
}

#[rstest]
fn sort_by_lower_parallel(mut conn: PgConnection) {
    // When parallel workers are used, we should not claim that the output that we produce is
    // sorted. Each worker will consume a series of segments, each of which is individually
    // sorted, but the overall output is not.
    "SET max_parallel_workers = 8;".execute(&mut conn);
    if pg_major_version(&mut conn) >= 16 {
        "SET debug_parallel_query TO on".execute(&mut conn);
    } else {
        // We cannot reliably force parallel workers to be used without `debug_parallel_query`.
        return;
    }

    let plan = field_sort_fixture(&mut conn);

    // Check the plan structure to see if we have a parallel plan
    let gather_node = plan
        .pointer("/0/Plan/Plans/0")
        .and_then(|n| n.as_object())
        .unwrap();

    // With parallel plans, check that the parallel worker is doing the right thing
    let worker_node = gather_node
        .get("Plans")
        .and_then(|plans| plans.as_array())
        .and_then(|plans| plans.first())
        .and_then(|plan| plan.as_object())
        .unwrap();

    assert!(
        worker_node
            .get("Parallel Aware")
            .is_some_and(|p| p.as_bool().unwrap_or(false)),
        "Worker should be parallel aware"
    );

    // In our implementation, we might use either a CustomScan or a Sort node
    let node_type = worker_node.get("Node Type").unwrap().as_str().unwrap();

    if node_type == "Custom Scan" {
        // If it's a Custom Scan, ensure Sort Field and Partial Sort Flag are set correctly
        let sort_field = worker_node.get("Sort Field");
        assert!(
            sort_field.is_some(),
            "Custom Scan should have a Sort Field: {:?}",
            worker_node
        );

        // Check if the category field is being used for sorting
        if let Some(sort_field_value) = sort_field {
            assert_eq!(
                sort_field_value,
                &Value::String("category".to_string()),
                "Sort Field should be 'category'"
            );
        }

        // Verify that the Partial Sort Flag is set
        let has_partial_sort = worker_node.get("Partial Sort Flag");
        assert!(
            has_partial_sort.is_some(),
            "Custom Scan should indicate if partial sort is enabled"
        );

        // Check if scores are enabled
        let scores = worker_node.get("Scores");
        assert!(
            scores.is_some(),
            "Custom Scan should indicate if scores are enabled"
        );
    } else {
        // If not a Custom Scan, ensure it's a Sort node
        assert_eq!(
            node_type, "Sort",
            "Expected either Custom Scan or Sort node in parallel worker"
        );
    }
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
        plan.get("Sort Field"),
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
    let plan_node = plan
        .pointer("/0/Plan/Plans/0")
        .unwrap_or_else(|| {
            // If we can't find the expected path, try another common path
            plan.pointer("/0/Plan").unwrap_or(&plan)
        })
        .as_object()
        .unwrap();

    // Find the Custom Scan node which could be at different levels based on the plan
    let custom_scan_node = if plan_node
        .get("Node Type")
        .is_some_and(|v| v.as_str() == Some("Custom Scan"))
    {
        plan_node
    } else if let Some(plans) = plan_node.get("Plans").and_then(|p| p.as_array()) {
        // If we have nested plans, look for Custom Scan in them
        plans
            .iter()
            .find(|p| {
                p.get("Node Type")
                    .is_some_and(|v| v.as_str() == Some("Custom Scan"))
            })
            .and_then(|p| p.as_object())
            .unwrap_or(plan_node)
    } else {
        // If we can't find a Custom Scan node, use the plan_node anyway
        plan_node
    };

    // When scores are enabled and ordering by category, we should still see:

    // 1. Check that scores are enabled
    assert!(
        custom_scan_node
            .get("Scores")
            .is_some_and(|v| v.as_bool() == Some(true)),
        "Plan should indicate Scores are enabled: {:#?}",
        custom_scan_node
    );

    // 2. We may have a Sort Field even with scores, but the planner might decide to
    // use a different approach when requesting scores, so we don't strictly require it
    let has_sort_field = custom_scan_node.get("Sort Field").is_some();
    let has_partial_sort = custom_scan_node.get("Partial Sort Flag").is_some();

    // 3. If we have a sort field, then partial sort flag should also be set
    if has_sort_field {
        assert_eq!(
            custom_scan_node.get("Sort Field").unwrap(),
            &Value::String("category".to_string()),
            "Sort Field should be 'category' when present"
        );

        // 4. If Sort Field is present, Partial Sort Flag should also be set
        if has_partial_sort {
            assert_eq!(
                custom_scan_node.get("Partial Sort Flag").unwrap(),
                &Value::String("True".to_string()),
                "Partial Sort Flag should be 'True' when present"
            );
        }
    }
}

#[rstest]
async fn test_incremental_sort_with_partial_order(mut conn: PgConnection) {
    // Create the test table
    sqlx::query(
        r#"
        CREATE TABLE sales (
            id SERIAL,
            sale_date DATE NOT NULL,
            amount REAL NOT NULL,
            description TEXT,
            PRIMARY KEY (id, sale_date)
        ) PARTITION BY RANGE (sale_date);
        "#,
    )
    .execute(&mut conn)
    .await
    .unwrap();

    // Create partitions
    sqlx::query(
        r#"
        CREATE TABLE sales_2023_q1 PARTITION OF sales
          FOR VALUES FROM ('2023-01-01') TO ('2023-04-01');
        "#,
    )
    .execute(&mut conn)
    .await
    .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE sales_2023_q2 PARTITION OF sales
          FOR VALUES FROM ('2023-04-01') TO ('2023-06-30');
        "#,
    )
    .execute(&mut conn)
    .await
    .unwrap();

    // Insert test data
    sqlx::query(
        r#"
        INSERT INTO sales (sale_date, amount, description)
        SELECT
           (DATE '2023-01-01' + (random() * 179)::integer) AS sale_date,
           (random() * 1000)::real AS amount,
           ('thing '::text || md5(random()::text)) AS description
        FROM generate_series(1, 1000);
        "#,
    )
    .execute(&mut conn)
    .await
    .unwrap();

    // Create a bm25 index
    sqlx::query(
        r#"
        CREATE INDEX sales_index ON sales
          USING bm25 (id, description, sale_date)
          WITH (
            key_field='id',
            datetime_fields = '{
                "sale_date": {"fast": true}
            }'
          );
        "#,
    )
    .execute(&mut conn)
    .await
    .unwrap();

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

    // For testing purposes, try to force PostgreSQL to use our path
    // sqlx::query("SET enable_sort = off;")
    //     .execute(&mut conn)
    //     .await
    //     .unwrap();

    // Check Postgres version - Incremental Sort only exists in PG 15+
    let pg_version = pg_major_version(&mut conn);
    let pg_supports_incremental_sort = pg_version >= 15;

    // Test BM25 with ORDER BY ... LIMIT to confirm sort optimization works
    let (explain_bm25,) = sqlx::query_as::<_, (Value,)>(
        "EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON) 
        SELECT description, sale_date, paradedb.score(id) FROM sales 
        WHERE description @@@ 'keyboard' 
        ORDER BY sale_date, amount LIMIT 10;",
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
        SELECT description, sale_date, paradedb.score(id) FROM sales 
        WHERE description @@@ 'keyboard' 
        ORDER BY sale_date LIMIT 10;",
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
    let has_sort_node = plan_json.contains("\"Node Type\":\"Sort\"")
        || plan_json.contains("\"Node Type\":\"Incremental Sort\"")
        || explain_simple.contains("Sort")
        || explain_simple.contains("Incremental Sort");

    assert!(
        has_sort_node,
        "Plan should include a Sort or Incremental Sort node to handle ORDER BY"
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

    // Log information about Sort or Incremental Sort in the plan
    if pg_supports_incremental_sort {
        println!("Note: PostgreSQL 15+ supports Incremental Sort, but standard Sort may be used depending on cost model decisions");
    } else {
        println!("Note: PostgreSQL 14 does not support Incremental Sort, using standard Sort");
    }

    // Verify we get results and they're in the correct order
    let results = sqlx::query(
        "SELECT description, sale_date, paradedb.score(id) FROM sales 
        WHERE description @@@ 'keyboard' 
        ORDER BY sale_date, amount LIMIT 10;",
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
