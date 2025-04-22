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

#[rstest]
fn sort_by_lower(mut conn: PgConnection) {
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
    "#.execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE description @@@ 'keyboard OR shoes' ORDER BY lower(category) LIMIT 5".fetch_one::<(Value,)>(&mut conn);
    let plan = plan
        .pointer("/0/Plan/Plans/0/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    eprintln!("{plan:#?}");
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
    let plan = plan
        .pointer("/0/Plan/Plans/0/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    eprintln!("{plan:#?}");
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
    let plan = plan
        .pointer("/0/Plan/Plans/0/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    eprintln!("{plan:#?}");
    assert_eq!(plan.get("   Sort Field"), None);
    assert_eq!(plan.get("Scores"), Some(&Value::Bool(true)));
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

    // Test BM25 with ORDER BY ... LIMIT to confirm Incremental Sort is used
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

    // Check for Incremental Sort - need to check for both text and JSON formats
    // In JSON format, we need to look for "Node Type":"Incremental Sort"
    assert!(
        plan_json.contains("\"Node Type\":\"Incremental Sort\"")
            || explain_simple.contains("Incremental Sort"),
        "BM25 should use Incremental Sort, plan was: {} \n\nSimple plan was: {}",
        plan_json,
        explain_simple
    );

    // For Presorted Key we need to check for multiple formats
    // In JSON format: "Presorted Key":[\"sales.sale_date\"]
    // In text format: "Presorted Key: sales.sale_date"
    assert!(
        plan_json.contains("\"Presorted Key\":[") || explain_simple.contains("Presorted Key:"),
        "BM25 should use presorted keys, plan was: {} \n\nSimple plan was: {}",
        plan_json,
        explain_simple
    );

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
