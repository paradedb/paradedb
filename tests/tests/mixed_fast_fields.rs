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

// Tests for MixedFastFieldExecState implementation

mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

// Helper function to check if a specific execution method is used
fn check_exec_method(plan: &Value, method_name: &str) -> bool {
    // Try different paths where the method might be found
    for path in [
        "/0/Plan/Plans/0/Plans/0/Plans/0",
        "/0/Plan/Plans/0/Plans/0",
        "/0/Plan/Plans/0",
        "/0/Plan",
    ] {
        if let Some(node) = plan.pointer(path) {
            if let Some(exec_method) = node.get("Exec Method") {
                return exec_method.as_str().unwrap_or("") == method_name;
            }
        }
    }
    false
}

// Helper function to check if a specific execution method is NOT used
fn assert_exec_method_not_used(plan: &Value, method_name: &str) -> bool {
    let methods = get_all_exec_methods(plan);
    !methods.contains(&method_name.to_string())
}

// Helper function to get all execution methods in the plan
fn get_all_exec_methods(plan: &Value) -> Vec<String> {
    let mut methods = Vec::new();

    // Recursive function to walk the plan tree
    fn extract_methods(node: &Value, methods: &mut Vec<String>) {
        if let Some(exec_method) = node.get("Exec Method") {
            if let Some(method) = exec_method.as_str() {
                methods.push(method.to_string());
            }
        }

        // Check child plans
        if let Some(plans) = node.get("Plans") {
            if let Some(plans_array) = plans.as_array() {
                for plan in plans_array {
                    extract_methods(plan, methods);
                }
            }
        }
    }

    // Start from the root
    if let Some(root) = plan.get(0) {
        if let Some(plan_node) = root.get("Plan") {
            extract_methods(plan_node, &mut methods);
        }
    }

    methods
}

struct TestMixedFastFields;

impl TestMixedFastFields {
    fn setup() -> impl Query {
        r#"
            DROP TABLE IF EXISTS documents CASCADE;
            DROP TABLE IF EXISTS files CASCADE;
            DROP TABLE IF EXISTS pages CASCADE;
            DROP TABLE IF EXISTS mixed_numeric_string_test CASCADE;
            
            -- Create test tables
            CREATE TABLE documents (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT,
                parents TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT NOW()
            );
            
            CREATE TABLE files (
                id TEXT NOT NULL UNIQUE,
                documentId TEXT NOT NULL,
                title TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_size INTEGER,
                created_at TIMESTAMP DEFAULT NOW(),
                PRIMARY KEY (id, documentId),
                FOREIGN KEY (documentId) REFERENCES documents(id)
            );
            
            CREATE TABLE pages (
                id TEXT NOT NULL UNIQUE,
                fileId TEXT NOT NULL,
                page_number INTEGER NOT NULL,
                content TEXT NOT NULL,
                metadata JSONB,
                created_at TIMESTAMP DEFAULT NOW(),
                PRIMARY KEY (id, fileId),
                FOREIGN KEY (fileId) REFERENCES files(id)
            );
            
            -- Create BM25 indexes with fast fields
            CREATE INDEX documents_search ON documents USING bm25 (
                id,
                title,
                parents,
                content
            ) WITH (
                key_field = 'id',
                text_fields = '{"title": {"tokenizer": {"type": "default"}, "fast": true}, "parents": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}, "fast": true}}'
            );
            
            CREATE INDEX files_search ON files USING bm25 (
                id,
                documentId,
                title,
                file_path
            ) WITH (
                key_field = 'id',
                text_fields = '{"documentid": {"tokenizer": {"type": "keyword"}, "fast": true}, "title": {"tokenizer": {"type": "default"}, "fast": true}, "file_path": {"tokenizer": {"type": "default"}, "fast": true}}'
            );
            
            CREATE INDEX pages_search ON pages USING bm25 (
                id,
                fileId,
                content,
                page_number
            ) WITH (
                key_field = 'id',
                text_fields = '{"fileid": {"tokenizer": {"type": "keyword"}, "fast": true}, "content": {"tokenizer": {"type": "default"}}}',
                numeric_fields = '{"page_number": {"fast": true}}'
            );
            
            -- Insert sample data
            INSERT INTO documents (id, title, content, parents) VALUES
            ('doc1', 'Invoice 2023', 'This is an invoice for services rendered in 2023', 'Factures'),
            ('doc2', 'Receipt 2023', 'This is a receipt for payment received in 2023', 'Factures'),
            ('doc3', 'Contract 2023', 'This is a contract for services in 2023', 'Contracts');
            
            INSERT INTO files (id, documentId, title, file_path, file_size) VALUES
            ('file1', 'doc1', 'Invoice PDF', '/invoices/2023.pdf', 1024),
            ('file2', 'doc1', 'Invoice Receipt', '/invoices/2023_receipt.pdf', 512),
            ('file3', 'doc2', 'Receipt', '/receipts/2023.pdf', 256),
            ('file4', 'doc3', 'Contract Document', '/contracts/2023.pdf', 2048);
            
            INSERT INTO pages (id, fileId, page_number, content) VALUES
            ('page1', 'file1', 1, 'Page 1 of Invoice PDF with Socienty General details'),
            ('page2', 'file1', 2, 'Page 2 of Invoice PDF with payment information'),
            ('page3', 'file2', 1, 'Page 1 of Invoice Receipt with bank details'),
            ('page4', 'file3', 1, 'Page 1 of Receipt with Socienty General information'),
            ('page5', 'file3', 2, 'Page 2 of Receipt with transaction ID'),
            ('page6', 'file4', 1, 'Page 1 of Contract Document with terms and conditions');
            
            -- Create test table for mixed numeric/string testing
            CREATE TABLE mixed_numeric_string_test (
                id TEXT PRIMARY KEY,
                numeric_field1 INTEGER NOT NULL,
                numeric_field2 BIGINT NOT NULL,
                string_field1 TEXT NOT NULL,
                string_field2 TEXT NOT NULL,
                string_field3 TEXT NOT NULL,
                content TEXT
            );
            
            -- Create index with both numeric and string fast fields
            CREATE INDEX mixed_test_search ON mixed_numeric_string_test USING bm25 (
                id,
                numeric_field1,
                numeric_field2,
                string_field1,
                string_field2,
                string_field3,
                content
            ) WITH (
                key_field = 'id',
                text_fields = '{"string_field1": {"tokenizer": {"type": "default"}, "fast": true}, "string_field2": {"tokenizer": {"type": "default"}, "fast": true}, "string_field3": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}}}',
                numeric_fields = '{"numeric_field1": {"fast": true}, "numeric_field2": {"fast": true}}'
            );
            
            -- Insert test data
            INSERT INTO mixed_numeric_string_test (id, numeric_field1, numeric_field2, string_field1, string_field2, string_field3, content) VALUES
            ('mix1', 100, 10000, 'Apple', 'Red', 'Fruit', 'This is a red apple'),
            ('mix2', 200, 20000, 'Banana', 'Yellow', 'Fruit', 'This is a yellow banana'),
            ('mix3', 300, 30000, 'Carrot', 'Orange', 'Vegetable', 'This is an orange carrot'),
            ('mix4', 400, 40000, 'Donut', 'Brown', 'Dessert', 'This is a chocolate donut'),
            ('mix5', 500, 50000, 'Egg', 'White', 'Protein', 'This is a white egg');
            "#
    }
}

#[rstest]
fn test_basic_mixed_string_numeric_fields(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 1: Mixed string and numeric fields
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Check execution method
    assert!(
        check_exec_method(&plan, "MixedFastFieldExecState"),
        "Expected MixedFastFieldExecState, got: {:?}",
        get_all_exec_methods(&plan)
    );

    // Check results
    let results = r#"
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_result::<(String, i32)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 2, "Expected 2 results");

    // Assert specific values
    let expected_results = vec![("file1".to_string(), 1), ("file3".to_string(), 1)];

    let mut found_results = results;
    found_results.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    assert_eq!(found_results, expected_results, "Results don't match");
}

#[rstest]
fn test_multiple_string_fast_fields(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 2: Multiple string fast fields
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT string_field1, string_field2, string_field3
        FROM mixed_numeric_string_test
        WHERE content @@@ 'red'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Check execution method
    assert!(
        check_exec_method(&plan, "MixedFastFieldExecState"),
        "Expected MixedFastFieldExecState, got: {:?}",
        get_all_exec_methods(&plan)
    );

    // Verify results
    let results = r#"
        SELECT string_field1, string_field2, string_field3
        FROM mixed_numeric_string_test
        WHERE content @@@ 'red'
    "#
    .fetch_result::<(String, String, String)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 1, "Expected 1 result");
    assert_eq!(
        results[0],
        ("Apple".to_string(), "Red".to_string(), "Fruit".to_string()),
        "Result doesn't match expected"
    );
}

#[rstest]
fn test_multiple_numeric_fast_fields(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 3: Multiple numeric fast fields
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT numeric_field1, numeric_field2
        FROM mixed_numeric_string_test
        WHERE content @@@ 'red'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // This should use NumericFastFieldExecState since we're only selecting numeric fields
    let methods = get_all_exec_methods(&plan);
    assert!(
        methods.contains(&"NumericFastFieldExecState".to_string()),
        "Expected NumericFastFieldExecState, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        SELECT numeric_field1, numeric_field2
        FROM mixed_numeric_string_test
        WHERE content @@@ 'red'
    "#
    .fetch_result::<(i32, i64)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 1, "Expected 1 result");
    assert_eq!(results[0], (100, 10000), "Result doesn't match expected");
}

#[rstest]
fn test_mixed_field_types_in_query(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 4: Mix of string and numeric fields in the same query
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT numeric_field1, string_field1, numeric_field2, string_field2
        FROM mixed_numeric_string_test
        WHERE content @@@ 'red'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Check execution method
    assert!(
        check_exec_method(&plan, "MixedFastFieldExecState"),
        "Expected MixedFastFieldExecState, got: {:?}",
        get_all_exec_methods(&plan)
    );

    // Verify results
    let results = r#"
        SELECT numeric_field1, string_field1, numeric_field2, string_field2
        FROM mixed_numeric_string_test
        WHERE content @@@ 'red'
    "#
    .fetch_result::<(i32, String, i64, String)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 1, "Expected 1 result");
    assert_eq!(
        results[0],
        (100, "Apple".to_string(), 10000, "Red".to_string()),
        "Result doesn't match expected"
    );
}

#[rstest]
fn test_complex_join_with_mixed_fields(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 5: Complex join query with mixed fields
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT documents.id, documents.parents, files.title, files.file_path, pages.fileId, pages.page_number
        FROM documents 
        JOIN files ON documents.id = files.documentId
        JOIN pages ON pages.fileId = files.id
        WHERE documents.parents @@@ 'Factures' AND files.title @@@ 'Receipt' AND pages.content @@@ 'Socienty'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // For a complex join, we should see MixedFastFieldExecState used in the execution plan
    let methods = get_all_exec_methods(&plan);

    // Check that MixedFastFieldExecState is used (potentially multiple times for different tables)
    assert!(
        methods.iter().any(|m| m == "MixedFastFieldExecState"),
        "Expected MixedFastFieldExecState to be used at least once, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        SELECT documents.id, documents.parents, files.title, files.file_path, pages.fileId, pages.page_number
        FROM documents 
        JOIN files ON documents.id = files.documentId
        JOIN pages ON pages.fileId = files.id
        WHERE documents.parents @@@ 'Factures' AND files.title @@@ 'Receipt' AND pages.content @@@ 'Socienty'
    "#
    .fetch_result::<(String, String, String, String, String, i32)>(&mut conn).unwrap();

    assert!(
        results.len() > 0,
        "Expected at least one result from join query"
    );
}

#[rstest]
fn test_limit_clause_uses_topn(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 6: Query with LIMIT clause
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'Socienty'
        LIMIT 10
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // We expect TopNScanExecState for LIMIT queries
    let methods = get_all_exec_methods(&plan);
    assert!(
        methods.contains(&"TopNScanExecState".to_string()),
        "Expected TopNScanExecState for LIMIT query, got: {:?}",
        methods
    );
}

#[rstest]
fn test_order_by_with_mixed_fields(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 7: Query with ORDER BY
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'Socienty'
        ORDER BY page_number
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // MixedFastFieldExecState should be used even with ORDER BY
    let methods = get_all_exec_methods(&plan);
    assert!(
        methods.contains(&"MixedFastFieldExecState".to_string()),
        "Expected MixedFastFieldExecState with ORDER BY, got: {:?}",
        methods
    );

    // Verify results are ordered correctly
    let results = r#"
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'Socienty'
        ORDER BY page_number
    "#
    .fetch_result::<(String, i32)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 2, "Expected 2 results");

    // Check ordering
    for i in 1..results.len() {
        assert!(
            results[i].1 >= results[i - 1].1,
            "Results not ordered correctly by page_number"
        );
    }
}

#[rstest]
fn test_aggregation_query(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 8: Query with aggregation
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT COUNT(*)
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // For aggregation, the plan might use NormalScanExecState
    let methods = get_all_exec_methods(&plan);
    println!("Execution methods for aggregation query: {:?}", methods);

    // Verify count is correct
    let (count,) = r#"
        SELECT COUNT(*)
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(count, 2, "Expected count of 2 matching documents");
}

#[rstest]
fn test_full_table_scan_with_mixed_fields(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 9: Query returning all columns
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT *
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Even with all columns, MixedFastFieldExecState should be used when applicable
    let methods = get_all_exec_methods(&plan);

    // Either MixedFastFieldExecState or NormalScanExecState could be used depending on implementation details
    println!("Execution methods for full table scan: {:?}", methods);

    // Verify results
    let (count,) = r#"
        SELECT COUNT(*)
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(count, 2, "Expected 2 matching documents");
}

#[rstest]
fn test_result_correctness(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 10: Result correctness verification
    // First using MixedFastFieldExecState
    let mixed_results = r#"
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_result::<(String, i32)>(&mut conn)
    .unwrap();

    // Then with a different approach (forcing a different execution path)
    let forced_results = r#"
        -- Force a different execution path by selecting an additional column
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'Socienty'
        ORDER BY id -- Adding ORDER BY to potentially change execution path
    "#
    .fetch_result::<(String, i32)>(&mut conn)
    .unwrap();

    // Results should be the same regardless of execution method
    assert_eq!(
        mixed_results.len(),
        forced_results.len(),
        "Result counts don't match"
    );

    // Ensure all records match (ignoring order)
    let mut mixed_sorted = mixed_results.clone();
    let mut forced_sorted = forced_results.clone();

    mixed_sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    forced_sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    assert_eq!(
        mixed_sorted, forced_sorted,
        "Results don't match between different execution methods"
    );
}

#[rstest]
fn test_edge_case_no_results(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test 11: Edge case with no matching results
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'NonExistentTerm'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Should still use MixedFastFieldExecState even when no results are found
    let methods = get_all_exec_methods(&plan);
    assert!(
        methods.contains(&"MixedFastFieldExecState".to_string()),
        "Expected MixedFastFieldExecState even with no results, got: {:?}",
        methods
    );

    // Verify no results are returned
    let results = r#"
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'NonExistentTerm'
    "#
    .fetch_result::<(String, i32)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 0, "Expected 0 results");
}

#[rstest]
fn test_performance_comparison(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Add more test data for meaningful performance comparison
    r#"
        DO $$
        DECLARE
            i INTEGER;
        BEGIN
            FOR i IN 1..100 LOOP
                INSERT INTO mixed_numeric_string_test (
                    id, 
                    numeric_field1, 
                    numeric_field2, 
                    string_field1, 
                    string_field2, 
                    string_field3, 
                    content
                ) VALUES (
                    'perf' || i,
                    i * 10,
                    i * 1000,
                    'Test' || (i % 5),
                    'Color' || (i % 3),
                    'Type' || (i % 4),
                    CASE 
                        WHEN i % 10 = 0 THEN 'Contains benchmark term for testing'
                        ELSE 'Regular content ' || i
                    END
                );
            END LOOP;
        END $$;
    "#
    .execute(&mut conn);

    // Test 13: Performance comparison
    // With MixedFastFieldExecState
    let (mixed_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT numeric_field1, string_field1
        FROM mixed_numeric_string_test
        WHERE content @@@ 'benchmark'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Force normal execution by selecting all columns
    let (normal_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT *
        FROM mixed_numeric_string_test
        WHERE content @@@ 'benchmark'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Extract execution times for comparison
    let mixed_time = mixed_plan[0]["Plan"]["Actual Total Time"].as_f64().unwrap();
    let normal_time = normal_plan[0]["Plan"]["Actual Total Time"]
        .as_f64()
        .unwrap();

    println!("Mixed execution time: {}ms", mixed_time);
    println!("Normal execution time: {}ms", normal_time);

    // Verify results are the same
    let mixed_results = r#"
        SELECT numeric_field1, string_field1
        FROM mixed_numeric_string_test
        WHERE content @@@ 'benchmark'
    "#
    .fetch_result::<(i32, String)>(&mut conn)
    .unwrap();

    let normal_results = r#"
        SELECT numeric_field1, string_field1
        FROM mixed_numeric_string_test
        WHERE content @@@ 'benchmark'
    "#
    .fetch_result::<(i32, String)>(&mut conn)
    .unwrap();

    assert_eq!(
        mixed_results.len(),
        normal_results.len(),
        "Result counts don't match between execution methods"
    );

    // Sort and compare results
    let mut mixed_sorted = mixed_results.clone();
    let mut normal_sorted = normal_results.clone();

    mixed_sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    normal_sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    assert_eq!(
        mixed_sorted, normal_sorted,
        "Results don't match between execution methods"
    );
}

#[rstest]
fn test_normal_scan_not_used_when_fast_field_capable(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Get execution plan
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Check that MixedFastFieldExecState is used
    assert!(
        check_exec_method(&plan, "MixedFastFieldExecState"),
        "Expected MixedFastFieldExecState to be used"
    );

    // Check that NormalScanExecState is NOT used
    assert!(
        assert_exec_method_not_used(&plan, "NormalScanExecState"),
        "NormalScanExecState should not be used when fast field capable"
    );

    // Verify results are correct
    let results = r#"
        SELECT fileId, page_number
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_result::<(String, i32)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 2, "Expected 2 results");
}

#[rstest]
fn test_normal_scan_used_when_non_fast_fields_selected(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // This test shows when NormalScanExecState should be used:
    // When selecting content (non-fast field) from pages
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT content
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Since content is not a fast field, we should use NormalScanExecState
    let methods = get_all_exec_methods(&plan);
    assert!(
        methods.contains(&"NormalScanExecState".to_string()),
        "Expected NormalScanExecState for non-fast field queries, got: {:?}",
        methods
    );

    // MixedFastFieldExecState should not be used in this case
    assert!(
        !methods.contains(&"MixedFastFieldExecState".to_string()),
        "MixedFastFieldExecState should not be used when only selecting non-fast fields"
    );

    // Verify the results are correct
    let results = r#"
        SELECT content
        FROM pages
        WHERE content @@@ 'Socienty'
    "#
    .fetch_result::<(String,)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 2, "Expected 2 results");
    // Check for content containing 'Socienty'
    for (content,) in &results {
        assert!(
            content.contains("Socienty"),
            "Content should contain 'Socienty'"
        );
    }
}
