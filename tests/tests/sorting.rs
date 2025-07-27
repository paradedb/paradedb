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

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

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
        plan.get("   Sort Field"),
        Some(&Value::String(String::from("category")))
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
    if pg_major_version(&mut conn) < 16 {
        // Incremental Sort is only supported >=16.
        return;
    }
    "SET max_parallel_workers to 0;".execute(&mut conn);

    // Create the partitioned sales table
    PartitionedTable::setup().execute(&mut conn);

    // Insert a good size amount of random data, and then analyze. Postgres will not choose an
    // Incremental Sort for smaller result sets, and we won't report a useful estimate without
    // statistics.
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

    // Test BM25 with ORDER BY ... LIMIT to confirm sort optimization works
    let (explain_bm25,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id, sale_date, amount FROM sales
        WHERE description @@@ 'wine'
        ORDER BY sale_date, amount LIMIT 10;"#
        .fetch_one(&mut conn);

    eprintln!("plan: {explain_bm25:#?}");

    let plan_json = explain_bm25.to_string();

    // Extract the Custom Scan nodes from the JSON plan for inspection
    let mut custom_scan_nodes = Vec::new();
    if let Ok(plan) = serde_json::from_str::<Value>(&plan_json) {
        // Navigate through the plan to find Custom Scan nodes
        if let Some(main_plan) = plan.pointer("/0/Plan") {
            collect_custom_scan_nodes(main_plan, &mut custom_scan_nodes);
        }
    }

    // Check that we have a Sort node somewhere in the plan
    assert!(
        plan_json.contains("\"Node Type\":\"Incremental Sort\""),
        "Plan should include an Incremental Sort node to handle ORDER BY"
    );

    // Check that we have Custom Scan nodes that handle our search
    assert_eq!(custom_scan_nodes.len(), 2);
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
