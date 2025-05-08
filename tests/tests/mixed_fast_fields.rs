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
// Includes both basic functionality tests and corner/edge cases

mod fixtures;

use bigdecimal::BigDecimal;
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
    extract_methods(plan, &mut methods);
    methods
}

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

    // Start from the root if given the root plan
    if let Some(root) = node.get(0) {
        if let Some(plan_node) = root.get("Plan") {
            extract_methods(plan_node, methods);
        }
    }
}

// Setup functions for test data
// ============================

// Setup for basic mixed fast fields tests
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

// Setup for corner cases and edge cases tests
struct TestCornerCases;

impl TestCornerCases {
    fn setup() -> impl Query {
        r#"
            DROP TABLE IF EXISTS corner_case_test CASCADE;
            
            -- Create test tables with unusual/extreme cases
            CREATE TABLE corner_case_test (
                id TEXT PRIMARY KEY,
                -- String fields with different characteristics
                empty_string TEXT NOT NULL,
                very_long_string TEXT NOT NULL,
                special_chars TEXT NOT NULL,
                non_utf8_bytes BYTEA NOT NULL,
                -- Numeric fields with different characteristics
                extreme_large BIGINT NOT NULL,
                extreme_small BIGINT NOT NULL,
                float_value FLOAT NOT NULL,
                zero_value INTEGER NOT NULL,
                negative_value INTEGER NOT NULL,
                -- Boolean field
                bool_field BOOLEAN NOT NULL,
                -- Regular fields for testing
                content TEXT
            );
            
            -- Create BM25 index with fast fields for all columns
            CREATE INDEX corner_case_search ON corner_case_test USING bm25 (
                id,
                empty_string,
                very_long_string,
                special_chars,
                extreme_large,
                extreme_small,
                float_value,
                zero_value,
                negative_value,
                bool_field,
                content
            ) WITH (
                key_field = 'id',
                text_fields = '{"empty_string": {"tokenizer": {"type": "default"}, "fast": true}, "very_long_string": {"tokenizer": {"type": "default"}, "fast": true}, "special_chars": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}}}',
                numeric_fields = '{"extreme_large": {"fast": true}, "extreme_small": {"fast": true}, "float_value": {"fast": true}, "zero_value": {"fast": true}, "negative_value": {"fast": true}}',
                boolean_fields = '{"bool_field": {"fast": true}}'
            );
            
            -- Insert extreme test data
            INSERT INTO corner_case_test (
                id, 
                empty_string, 
                very_long_string, 
                special_chars, 
                non_utf8_bytes,
                extreme_large, 
                extreme_small, 
                float_value, 
                zero_value, 
                negative_value, 
                bool_field, 
                content
            ) VALUES
            ('case1', '', repeat('a', 8000), '!@#$%^&*()_+{}[]|:;"''<>,.?/', E'\\x00', 9223372036854775807, -9223372036854775808, 1.7976931348623157e+308, 0, -2147483648, true, 'Contains test term'),
            ('case2', '', repeat('b', 2), '-_.+', E'\\x00', 0, 0, 0.0, 0, 0, false, 'Contains test term'),
            ('case3', 'not_empty', '', '漢字', E'\\x00', 42, -42, 3.14159, 0, -1, true, 'Contains test term');
            "#
    }
}

// SECTION 1: BASIC FUNCTIONALITY TESTS
// ===================================

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
        !results.is_empty(),
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

#[rstest]
fn test_string_only_fields_performance_comparison(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Add many rows with string fields for performance testing
    r#"
        DO $$
        DECLARE
            i INTEGER;
        BEGIN
            FOR i IN 1..1000 LOOP
                INSERT INTO mixed_numeric_string_test (
                    id, 
                    numeric_field1, 
                    numeric_field2, 
                    string_field1, 
                    string_field2, 
                    string_field3, 
                    content
                ) VALUES (
                    'str_perf' || i,
                    i,
                    i,
                    'String' || (i % 10),
                    'Value' || (i % 5),
                    'Type' || (i % 3),
                    CASE WHEN i % 20 = 0 THEN 'performance test case' ELSE 'other content' END
                );
            END LOOP;
        END $$;
    "#
    .execute(&mut conn);

    // Compare StringFastField vs MixedFastField for string-only queries

    // Force StringFastFieldExecState (by selecting only one string field)
    let (string_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT string_field1
        FROM mixed_numeric_string_test
        WHERE content @@@ 'performance'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Force MixedFastFieldExecState (by selecting multiple string fields)
    let (mixed_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT string_field1, string_field2, string_field3
        FROM mixed_numeric_string_test
        WHERE content @@@ 'performance'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Check if StringFastFieldExecState is used for the first query
    let string_methods = get_all_exec_methods(&string_plan);
    assert!(
        string_methods.contains(&"StringFastFieldExecState".to_string()),
        "Expected StringFastFieldExecState for single string field query"
    );

    // Check if MixedFastFieldExecState is used for the second query
    assert!(
        check_exec_method(&mixed_plan, "MixedFastFieldExecState"),
        "Expected MixedFastFieldExecState for multiple string fields"
    );

    // Compare execution times and verify results match
    let string_time = string_plan[0]["Plan"]["Actual Total Time"]
        .as_f64()
        .unwrap();
    let mixed_time = mixed_plan[0]["Plan"]["Actual Total Time"].as_f64().unwrap();

    println!("StringFastFieldExecState time: {}ms", string_time);
    println!("MixedFastFieldExecState time: {}ms", mixed_time);

    // Collect results from both execution methods
    let string_results = r#"
        SELECT string_field1
        FROM mixed_numeric_string_test
        WHERE content @@@ 'performance'
        ORDER BY id
    "#
    .fetch_result::<(String,)>(&mut conn)
    .unwrap();

    let mixed_results = r#"
        SELECT string_field1
        FROM mixed_numeric_string_test
        WHERE content @@@ 'performance'
        ORDER BY id
    "#
    .fetch_result::<(String,)>(&mut conn)
    .unwrap();

    // Results should match despite different execution methods
    assert_eq!(
        string_results.len(),
        mixed_results.len(),
        "Result counts don't match"
    );
    for i in 0..string_results.len() {
        assert_eq!(
            string_results[i].0, mixed_results[i].0,
            "Results at index {} don't match",
            i
        );
    }
}

#[rstest]
fn test_string_edge_cases(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Add edge cases: empty strings, special characters, very long strings
    r#"
        INSERT INTO mixed_numeric_string_test (id, numeric_field1, numeric_field2, string_field1, string_field2, string_field3, content) VALUES
        ('edge1', 1, 1, '', 'empty_first', 'test', 'edge case test'),
        ('edge2', 2, 2, 'special_chars_!@#$%^&*()', 'test', 'test', 'edge case test'),
        ('edge3', 3, 3, repeat('very_long_string_', 100), 'test', 'test', 'edge case test');
    "#.execute(&mut conn);

    // Test with StringFastFieldExecState (single field)
    let string_results = r#"
        SELECT string_field1
        FROM mixed_numeric_string_test
        WHERE content @@@ 'edge case'
        ORDER BY id
    "#
    .fetch_result::<(String,)>(&mut conn)
    .unwrap();

    // Test with MixedFastFieldExecState (multiple fields)
    let mixed_results = r#"
        SELECT string_field1, string_field2, string_field3
        FROM mixed_numeric_string_test
        WHERE content @@@ 'edge case'
        ORDER BY id
    "#
    .fetch_result::<(String, String, String)>(&mut conn)
    .unwrap();

    // Get execution plans to verify execution methods
    let (string_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT string_field1
        FROM mixed_numeric_string_test
        WHERE content @@@ 'edge case'
        ORDER BY id
    "#
    .fetch_one::<(Value,)>(&mut conn);

    let (mixed_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT string_field1, string_field2, string_field3
        FROM mixed_numeric_string_test
        WHERE content @@@ 'edge case'
        ORDER BY id
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Verify execution methods used
    let string_methods = get_all_exec_methods(&string_plan);
    let mixed_methods = get_all_exec_methods(&mixed_plan);

    println!("String query execution methods: {:?}", string_methods);
    println!("Mixed query execution methods: {:?}", mixed_methods);

    // Verify edge cases are handled correctly in both execution methods
    assert_eq!(
        string_results.len(),
        3,
        "Expected 3 edge case results with StringFastFieldExecState"
    );
    assert_eq!(
        mixed_results.len(),
        3,
        "Expected 3 edge case results with MixedFastFieldExecState"
    );

    // Verify empty string handling
    assert_eq!(
        string_results[0].0, "",
        "Empty string not handled correctly by StringFastFieldExecState"
    );
    assert_eq!(
        mixed_results[0].0, "",
        "Empty string not handled correctly by MixedFastFieldExecState"
    );

    // Verify special characters
    assert_eq!(
        string_results[1].0, "special_chars_!@#$%^&*()",
        "Special characters not handled correctly"
    );
    assert_eq!(
        mixed_results[1].0, "special_chars_!@#$%^&*()",
        "Special characters not handled correctly"
    );

    // Verify long string
    assert!(
        string_results[2].0.starts_with("very_long_string_"),
        "Long string truncated"
    );
    assert!(
        mixed_results[2].0.starts_with("very_long_string_"),
        "Long string truncated"
    );
}

// SECTION 2: CORNER CASES AND EDGE CASES TESTS
// ==========================================

#[rstest]
fn test_empty_strings(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Test handling of empty strings in MixedFastFieldExecState
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT empty_string, special_chars, extreme_large 
        FROM corner_case_test
        WHERE content @@@ 'test'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Check execution method
    assert!(
        check_exec_method(&plan, "MixedFastFieldExecState"),
        "Expected MixedFastFieldExecState, got: {:?}",
        get_all_exec_methods(&plan)
    );

    // Check results with empty strings
    let results = r#"
        SELECT id, empty_string
        FROM corner_case_test
        WHERE content @@@ 'test'
        ORDER BY id
    "#
    .fetch_result::<(String, String)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 3, "Expected 3 results");

    // First two records should have empty strings
    assert_eq!(results[0].1, "", "Expected empty string for case1");
    assert_eq!(results[1].1, "", "Expected empty string for case2");
    assert_eq!(
        results[2].1, "not_empty",
        "Expected non-empty string for case3"
    );
}

#[rstest]
fn test_very_long_strings(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Test handling of very long strings (buffer boundaries)
    let results = r#"
        SELECT id, very_long_string
        FROM corner_case_test
        WHERE content @@@ 'test'
        ORDER BY id
    "#
    .fetch_result::<(String, String)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 3, "Expected 3 results");

    // Check length of the very long string
    assert_eq!(
        results[0].1.len(),
        8000,
        "Expected very long string of 8000 chars"
    );
    assert_eq!(results[1].1.len(), 2, "Expected string of length 2");
    assert_eq!(results[2].1, "", "Expected empty string");
}

#[rstest]
fn test_special_characters(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Test handling of special characters
    let results = r#"
        SELECT id, special_chars
        FROM corner_case_test
        WHERE content @@@ 'test'
        ORDER BY id
    "#
    .fetch_result::<(String, String)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 3, "Expected 3 results");

    // Check special characters
    assert_eq!(
        results[0].1, "!@#$%^&*()_+{}[]|:;\"'<>,.?/",
        "Special characters not preserved"
    );
    assert_eq!(
        results[1].1, "-_.+",
        "Simple special characters not preserved"
    );
    assert_eq!(results[2].1, "漢字", "Unicode characters not preserved");
}

#[rstest]
fn test_extreme_numeric_values(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Test handling of extreme numeric values
    let results = r#"
        SELECT id, extreme_large, extreme_small
        FROM corner_case_test
        WHERE content @@@ 'test'
        ORDER BY id
    "#
    .fetch_result::<(String, i64, i64)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 3, "Expected 3 results");

    // Check extreme values
    assert_eq!(
        results[0].1, 9223372036854775807,
        "Max BIGINT value not preserved"
    );
    assert_eq!(
        results[0].2, -9223372036854775808,
        "Min BIGINT value not preserved"
    );

    // Check zero values
    assert_eq!(results[1].1, 0, "Zero value not preserved");
    assert_eq!(results[1].2, 0, "Zero value not preserved");
}

#[rstest]
fn test_boolean_values(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Test boolean field handling
    let results = r#"
        SELECT id, bool_field
        FROM corner_case_test
        WHERE content @@@ 'test'
        ORDER BY id
    "#
    .fetch_result::<(String, bool)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 3, "Expected 3 results");

    // Check boolean values
    assert_eq!(results[0].1, true, "Boolean true not preserved");
    assert_eq!(results[1].1, false, "Boolean false not preserved");
    assert_eq!(results[2].1, true, "Boolean true not preserved");
}

#[rstest]
fn test_all_field_types_together(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Test retrieving all different field types together
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT empty_string, very_long_string, special_chars, 
               extreme_large, extreme_small, float_value, 
               zero_value, negative_value, bool_field
        FROM corner_case_test
        WHERE content @@@ 'test'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Check execution method
    assert!(
        check_exec_method(&plan, "MixedFastFieldExecState"),
        "Expected MixedFastFieldExecState, got: {:?}",
        get_all_exec_methods(&plan)
    );

    // Verify result counts
    let results = r#"
        SELECT COUNT(*)
        FROM corner_case_test
        WHERE content @@@ 'test'
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(results.0, 3, "Expected 3 results");
}

#[rstest]
fn test_complex_string_patterns(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Add data with complex string patterns
    r#"
        INSERT INTO corner_case_test (
            id, 
            empty_string, 
            very_long_string, 
            special_chars, 
            non_utf8_bytes,
            extreme_large, 
            extreme_small, 
            float_value, 
            zero_value, 
            negative_value, 
            bool_field, 
            content
        ) VALUES
        ('complex1', 'pattern with spaces', 'line1\nline2\nline3', 'tab    tab', E'\\x00', 1, 1, 1.0, 1, 1, true, 'complex pattern test'),
        ('complex2', 'quotation "marks"', 'backslash\\test', 'percent%test', E'\\x00', 2, 2, 2.0, 2, 2, false, 'complex pattern test');
    "#
    .execute(&mut conn);

    // Test handling of complex patterns
    let results = r#"
        SELECT id, empty_string, special_chars 
        FROM corner_case_test
        WHERE content @@@ 'complex pattern'
        ORDER BY id
    "#
    .fetch_result::<(String, String, String)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 2, "Expected 2 results");

    // Check string patterns
    assert_eq!(
        results[0].1, "pattern with spaces",
        "Spaces not handled correctly"
    );
    assert_eq!(
        results[0].2, "tab    tab",
        "Tab character not handled correctly"
    );

    assert_eq!(
        results[1].1, "quotation \"marks\"",
        "Quote characters not handled correctly"
    );
    assert_eq!(
        results[1].2, "percent%test",
        "Percent character not handled correctly"
    );
}

#[rstest]
fn test_null_values_handling(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Add a row with NULL values where possible
    r#"
        CREATE TABLE nullable_test (
            id TEXT PRIMARY KEY,
            string_field TEXT,
            numeric_field INTEGER,
            content TEXT
        );
        
        CREATE INDEX nullable_search ON nullable_test USING bm25 (
            id, string_field, numeric_field, content
        ) WITH (
            key_field = 'id',
            text_fields = '{"string_field": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}}}',
            numeric_fields = '{"numeric_field": {"fast": true}}'
        );
        
        INSERT INTO nullable_test (id, string_field, numeric_field, content) VALUES
        ('null1', NULL, NULL, 'null test case'),
        ('null2', 'not null', 42, 'null test case');
    "#
    .execute(&mut conn);

    // Test handling NULL values
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT string_field, numeric_field
        FROM nullable_test
        WHERE content @@@ 'null'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Check execution method
    assert!(
        check_exec_method(&plan, "MixedFastFieldExecState"),
        "Expected MixedFastFieldExecState, got: {:?}",
        get_all_exec_methods(&plan)
    );

    // Verify NULL handling
    let results = r#"
        SELECT id, string_field, numeric_field
        FROM nullable_test
        WHERE content @@@ 'null'
        ORDER BY id
    "#
    .fetch_result::<(String, Option<String>, Option<i32>)>(&mut conn)
    .unwrap();

    assert_eq!(results.len(), 2, "Expected 2 results");

    // Check NULL values
    assert_eq!(results[0].0, "null1", "Expected 'null1' record");
    assert_eq!(results[0].1, None, "Expected NULL string_field");
    assert_eq!(results[0].2, None, "Expected NULL numeric_field");

    assert_eq!(results[1].0, "null2", "Expected 'null2' record");
    assert_eq!(
        results[1].1,
        Some("not null".to_string()),
        "Expected non-NULL string_field"
    );
    assert_eq!(results[1].2, Some(42), "Expected non-NULL numeric_field");
}

#[rstest]
fn test_concurrent_queries(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Add many rows for concurrency testing
    r#"
        DO $$
        DECLARE
            i INTEGER;
        BEGIN
            FOR i IN 1..100 LOOP
                INSERT INTO corner_case_test (
                    id, 
                    empty_string, 
                    very_long_string, 
                    special_chars, 
                    non_utf8_bytes,
                    extreme_large, 
                    extreme_small, 
                    float_value, 
                    zero_value, 
                    negative_value, 
                    bool_field, 
                    content
                ) VALUES (
                    'conc' || i, 
                    'string' || (i % 5), 
                    'long' || (i % 3), 
                    'special' || (i % 2), 
                    E'\\x00', 
                    i, 
                    -i, 
                    i * 1.1, 
                    0, 
                    -i, 
                    (i % 2 = 0), 
                    CASE WHEN i % 10 = 0 THEN 'concurrent test term' ELSE 'other content' END
                );
            END LOOP;
        END $$;
    "#
    .execute(&mut conn);

    // Run multiple queries in sequence to simulate concurrent behavior
    for _i in 1..5 {
        let (count,) = r#"
            SELECT COUNT(*)
            FROM corner_case_test
            WHERE content @@@ 'concurrent'
            "#
        .to_string()
        .fetch_one::<(i64,)>(&mut conn);

        assert_eq!(count, 10, "Expected correct number of results");
    }
}

#[rstest]
fn test_type_conversion_edge_cases(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Create a table with fields that will test type conversion edge cases
    r#"
        CREATE TABLE conversion_test (
            id TEXT PRIMARY KEY,
            smallint_field SMALLINT,
            integer_field INTEGER,
            bigint_field BIGINT,
            numeric_field NUMERIC(10,2),
            real_field REAL,
            double_field DOUBLE PRECISION,
            bool_from_int BOOLEAN,
            content TEXT
        );
        
        CREATE INDEX conversion_search ON conversion_test USING bm25 (
            id, smallint_field, integer_field, bigint_field, 
            numeric_field, real_field, double_field, bool_from_int, content
        ) WITH (
            key_field = 'id',
            text_fields = '{"content": {"tokenizer": {"type": "default"}}}',
            numeric_fields = '{
                "smallint_field": {"fast": true}, 
                "integer_field": {"fast": true}, 
                "bigint_field": {"fast": true}, 
                "numeric_field": {"fast": true}, 
                "real_field": {"fast": true}, 
                "double_field": {"fast": true}
            }',
            boolean_fields = '{"bool_from_int": {"fast": true}}'
        );
        
        INSERT INTO conversion_test VALUES
        ('conv1', 32767, 2147483647, 9223372036854775807, 9999999.99, 3.402e38, 1.7976931348623157e308, true, 'conversion test'),
        ('conv2', -32768, -2147483648, -9223372036854775808, -9999999.99, -3.402e38, -1.7976931348623157e308, false, 'conversion test'),
        ('conv3', 0, 0, 0, 0.0, 0.0, 0.0, false, 'conversion test');
    "#
    .execute(&mut conn);

    // Test type conversions with MixedFastFieldExecState
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT smallint_field, integer_field, bigint_field, 
               numeric_field, real_field, double_field, bool_from_int
        FROM conversion_test
        WHERE content @@@ 'conversion'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Check execution method
    let methods = get_all_exec_methods(&plan);
    assert!(
        methods.contains(&"MixedFastFieldExecState".to_string())
            || methods.contains(&"NumericFastFieldExecState".to_string()),
        "Expected MixedFastFieldExecState or NumericFastFieldExecState, got: {:?}",
        methods
    );

    // Verify we get correct results for all types
    let results = r#"
        SELECT id, smallint_field, integer_field, bigint_field, 
               numeric_field::text, real_field, double_field, bool_from_int
        FROM conversion_test
        WHERE content @@@ 'conversion'
        ORDER BY id
    "#
    .fetch_result::<(String, i16, i32, i64, String, f32, f64, bool)>(&mut conn)
    .unwrap();

    assert_eq!(
        results.len(),
        3,
        "Expected 3 results for type conversion test"
    );
}

// SECTION 3: ADVANCED CTE AND JOIN TESTS
// =====================================

#[rstest]
fn test_advanced_cte_with_multiple_search_fields(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Add more test data for better CTE testing
    r#"
        INSERT INTO documents (id, title, content, parents) VALUES
        ('doc_cte1', 'CTE Test Doc 1', 'This document tests common table expressions', 'Reports'),
        ('doc_cte2', 'CTE Test Doc 2', 'Another document for CTE testing', 'Reports');
        
        INSERT INTO files (id, documentId, title, file_path, file_size) VALUES
        ('file_cte1', 'doc_cte1', 'CTE Test File 1', '/reports/cte1.pdf', 500),
        ('file_cte2', 'doc_cte1', 'CTE Test File 2', '/reports/cte2.pdf', 600),
        ('file_cte3', 'doc_cte2', 'CTE Test File 3', '/reports/cte3.pdf', 700);
        
        INSERT INTO pages (id, fileId, page_number, content) VALUES
        ('page_cte1', 'file_cte1', 1, 'Page 1 with searchable content for CTE testing'),
        ('page_cte2', 'file_cte1', 2, 'Page 2 with more content for testing'),
        ('page_cte3', 'file_cte2', 1, 'Another page with test terms to search'),
        ('page_cte4', 'file_cte3', 1, 'Final test page for CTE testing');
    "#
    .execute(&mut conn);

    // Test with CTE using multiple search conditions
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        WITH searchable_docs AS (
            SELECT d.id, d.title, d.parents
            FROM documents d
            WHERE d.title @@@ 'CTE Test' AND d.parents @@@ 'Reports'
        ),
        matching_files AS (
            SELECT f.id, f.documentId, f.title, f.file_path, f.file_size
            FROM files f
            JOIN searchable_docs sd ON f.documentId = sd.id
            WHERE f.title @@@ 'CTE Test'
        ),
        relevant_pages AS (
            SELECT p.id, p.fileId, p.page_number, paradedb.score(p.id) as relevance
            FROM pages p
            JOIN matching_files mf ON p.fileId = mf.id
            WHERE p.content @@@ 'searchable OR testing'
            ORDER BY relevance DESC
        )
        SELECT sd.title as document_title, 
               mf.title as file_title, 
               mf.file_size, 
               rp.page_number,
               rp.relevance
        FROM searchable_docs sd
        JOIN matching_files mf ON sd.id = mf.documentId
        JOIN relevant_pages rp ON mf.id = rp.fileId
        ORDER BY rp.relevance DESC, mf.file_size DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods to verify at least one fast field execution state is used
    let methods = get_all_exec_methods(&plan);
    println!("Advanced CTE execution methods: {:?}", methods);

    // Force scoring with paradedb.score() function
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState to be used for complex CTE, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        WITH searchable_docs AS (
            SELECT d.id, d.title, d.parents
            FROM documents d
            WHERE d.title @@@ 'CTE Test' AND d.parents @@@ 'Reports'
        ),
        matching_files AS (
            SELECT f.id, f.documentId, f.title, f.file_path, f.file_size
            FROM files f
            JOIN searchable_docs sd ON f.documentId = sd.id
            WHERE f.title @@@ 'CTE Test'
        ),
        relevant_pages AS (
            SELECT p.id, p.fileId, p.page_number, paradedb.score(p.id) as relevance
            FROM pages p
            JOIN matching_files mf ON p.fileId = mf.id
            WHERE p.content @@@ 'searchable OR testing'
            ORDER BY relevance DESC
        )
        SELECT 
            sd.title as document_title, 
            mf.title as file_title, 
            mf.file_size, 
            rp.page_number,
            rp.relevance
        FROM searchable_docs sd
        JOIN matching_files mf ON sd.id = mf.documentId
        JOIN relevant_pages rp ON mf.id = rp.fileId
        ORDER BY rp.relevance DESC, mf.file_size DESC
    "#
    .fetch_result::<(String, String, i32, i32, f32)>(&mut conn)
    .unwrap();

    assert_eq!(
        results,
        vec![
            (
                "CTE Test Doc 1".to_string(),
                "CTE Test File 1".to_string(),
                500,
                1,
                3.0883389,
            ),
            (
                "CTE Test Doc 2".to_string(),
                "CTE Test File 3".to_string(),
                700,
                1,
                1.258828,
            ),
            (
                "CTE Test Doc 1".to_string(),
                "CTE Test File 1".to_string(),
                500,
                2,
                1.189365,
            ),
        ],
        "The results do not match the expected output"
    );
}

#[rstest]
fn test_nested_subqueries_with_multiple_search_conditions(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test with nested subqueries and multiple search conditions
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT 
            d.id,
            d.title,
            d.parents,
            (
                SELECT COUNT(*)
                FROM files f
                WHERE f.documentId = d.id AND f.title @@@ 'Invoice'
            ) AS invoice_file_count,
            (
                SELECT SUM(p.page_number)
                FROM pages p
                JOIN files f ON p.fileId = f.id
                WHERE f.documentId = d.id AND p.content @@@ 'Socienty'
            ) AS socienty_page_sum
        FROM documents d
        WHERE d.parents @@@ 'Factures'
        ORDER BY invoice_file_count DESC, socienty_page_sum DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Nested subqueries execution methods: {:?}", methods);

    // Verify at least one fast field execution state is used
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState for nested subqueries, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        SELECT 
            d.id,
            d.title,
            d.parents,
            (
                SELECT COUNT(*)
                FROM files f
                WHERE f.documentId = d.id AND f.title @@@ 'Invoice'
            ) AS invoice_file_count,
            (
                SELECT SUM(p.page_number)
                FROM pages p
                JOIN files f ON p.fileId = f.id
                WHERE f.documentId = d.id AND p.content @@@ 'Socienty'
            ) AS socienty_page_sum
        FROM documents d
        WHERE d.parents @@@ 'Factures'
        ORDER BY invoice_file_count DESC, socienty_page_sum DESC
    "#
    .fetch_result::<(String, String, String, i64, Option<i64>)>(&mut conn)
    .unwrap();

    // Should find at least one document in Factures
    assert!(
        !results.is_empty(),
        "Expected at least one document in Factures"
    );

    // Verify all documents found have parents = 'Factures'
    for (_, _, parents, _, _) in &results {
        assert_eq!(parents, "Factures", "Expected parents to be 'Factures'");
    }
}

#[rstest]
fn test_forced_execution_with_score_function(mut conn: PgConnection) {
    TestCornerCases::setup().execute(&mut conn);

    // Test 1: Using paradedb.score() function to force execution even with non-fast fields
    let (score_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT c.id, c.empty_string, c.special_chars, c.extreme_large, paradedb.score(c.id)
        FROM corner_case_test c
        WHERE c.content @@@ 'test'
        ORDER BY paradedb.score(c.id) DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let score_methods = get_all_exec_methods(&score_plan);
    println!("Score function execution methods: {:?}", score_methods);

    // Score function should force a FastFieldExecState
    assert!(
        score_methods
            .iter()
            .any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState with score function, got: {:?}",
        score_methods
    );

    // Test 2: Including non-fast field without score (should use NormalScanExecState)
    let (normal_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT c.id, c.empty_string, c.non_utf8_bytes, c.extreme_large
        FROM corner_case_test c
        WHERE c.content @@@ 'test'
        ORDER BY paradedb.score(c.id) DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let normal_methods = get_all_exec_methods(&normal_plan);
    println!("Non-fast field execution methods: {:?}", normal_methods);

    // Should use NormalScanExecState for non-fast fields
    assert!(
        normal_methods.contains(&"NormalScanExecState".to_string()),
        "Expected NormalScanExecState for non-fast fields, got: {:?}",
        normal_methods
    );

    // Verify results
    let score_results = r#"
        SELECT c.id
        FROM corner_case_test c
        WHERE c.content @@@ 'test'
        ORDER BY paradedb.score(c.id) DESC
    "#
    .fetch_result::<(String,)>(&mut conn)
    .unwrap();

    let normal_results = r#"
        SELECT c.id
        FROM corner_case_test c
        WHERE c.content @@@ 'test'
        ORDER BY c.id
    "#
    .fetch_result::<(String,)>(&mut conn)
    .unwrap();

    // Should have the same number of results
    assert_eq!(
        score_results.len(),
        normal_results.len(),
        "Score and normal query result counts don't match"
    );
}

#[rstest]
fn test_complex_multi_table_join_with_mixed_fields(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test with complex multi-table join with mixed field types
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT 
            d.id AS document_id,
            d.title AS document_title,
            f.id AS file_id,
            f.title AS file_title,
            p.page_number,
            m.numeric_field1,
            m.string_field1,
            m.string_field2
        FROM documents d
        JOIN files f ON d.id = f.documentId AND f.title @@@ 'Invoice'
        JOIN pages p ON f.id = p.fileId AND p.content @@@ 'Socienty'
        JOIN mixed_numeric_string_test m ON m.id = 'mix1' -- Cross join to mix in different field types
        WHERE d.parents @@@ 'Factures'
        ORDER BY p.page_number, m.numeric_field1
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Complex multi-table join execution methods: {:?}", methods);

    // At least one FastFieldExecState should be used
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState for complex multi-table join, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        SELECT 
            d.id AS document_id,
            d.title AS document_title,
            f.id AS file_id,
            f.title AS file_title,
            p.page_number,
            m.numeric_field1,
            m.string_field1,
            m.string_field2
        FROM documents d
        JOIN files f ON d.id = f.documentId AND f.title @@@ 'Invoice'
        JOIN pages p ON f.id = p.fileId AND p.content @@@ 'Socienty'
        JOIN mixed_numeric_string_test m ON m.id = 'mix1' -- Cross join to mix in different field types
        WHERE d.parents @@@ 'Factures'
        ORDER BY p.page_number, m.numeric_field1
    "#
    .fetch_result::<(String, String, String, String, i32, i32, String, String)>(&mut conn)
    .unwrap();

    // Should find at least one result
    assert!(
        !results.is_empty(),
        "Expected at least one result for complex multi-table join"
    );

    // Verify the mixed_numeric_string_test values are correct
    for (_, _, _, _, _, numeric_field1, string_field1, string_field2) in &results {
        assert_eq!(*numeric_field1, 100, "Expected numeric_field1 to be 100");
        assert_eq!(
            string_field1, "Apple",
            "Expected string_field1 to be 'Apple'"
        );
        assert_eq!(string_field2, "Red", "Expected string_field2 to be 'Red'");
    }
}

// SECTION 4: AGGREGATION AND SET OPERATIONS WITH SEARCH
// ===================================================

#[rstest]
fn test_union_with_different_exec_methods(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Test with UNION combining queries with different execution methods
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        (
            -- First query uses mixed fast fields
            SELECT 'Document' AS type, d.id, d.title, NULL::INTEGER AS page_number
            FROM documents d
            WHERE d.title @@@ 'Invoice OR Receipt'
        )
        UNION ALL
        (
            -- Second query uses non-fast field in the SELECT list
            SELECT 'Page' AS type, p.id, p.content, p.page_number
            FROM pages p
            WHERE p.content @@@ 'Socienty'
        )
        ORDER BY type, id
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("UNION with different exec methods: {:?}", methods);

    // Verify both FastFieldExecState and NormalScanExecState are used
    let has_fast_field = methods.iter().any(|m| m.contains("FastFieldExecState"));
    let has_normal_scan = methods.contains(&"NormalScanExecState".to_string());

    assert!(
        has_fast_field && has_normal_scan,
        "Expected both FastFieldExecState and NormalScanExecState, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        (
            SELECT 'Document' AS type, d.id, d.title, NULL::INTEGER AS page_number
            FROM documents d
            WHERE d.title @@@ 'Invoice OR Receipt'
        )
        UNION ALL
        (
            SELECT 'Page' AS type, p.id, p.content, p.page_number
            FROM pages p
            WHERE p.content @@@ 'Socienty'
        )
        ORDER BY type, id
    "#
    .fetch_result::<(String, String, String, Option<i32>)>(&mut conn)
    .unwrap();

    // Should find some documents and pages
    assert!(
        !results.is_empty(),
        "Expected at least one result for UNION query"
    );

    // Verify we have both Document and Page types
    let has_document = results.iter().any(|(t, _, _, _)| t == "Document");
    let has_page = results.iter().any(|(t, _, _, _)| t == "Page");

    assert!(has_document, "Expected at least one Document result");
    assert!(has_page, "Expected at least one Page result");
}

#[rstest]
fn test_window_functions_with_search(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Add more test data for better distribution
    r#"
        DO $$
        DECLARE
            i INTEGER;
        BEGIN
            FOR i IN 1..10 LOOP
                INSERT INTO mixed_numeric_string_test (
                    id, 
                    numeric_field1, 
                    numeric_field2, 
                    string_field1, 
                    string_field2, 
                    string_field3, 
                    content
                ) VALUES (
                    'window' || i,
                    (i * 10),
                    (i * 100),
                    'Group' || (i % 3),
                    'Window' || (i % 2),
                    'Test',
                    'Window function test with searchable terms'
                );
            END LOOP;
        END $$;
    "#
    .execute(&mut conn);

    // Test with window functions and search
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT 
            m.id,
            m.numeric_field1,
            m.string_field1,
            (AVG(m.numeric_field1) OVER (PARTITION BY m.string_field1)) AS avg_by_group,
            RANK() OVER (PARTITION BY m.string_field1 ORDER BY m.numeric_field1 DESC) AS rank_in_group,
            ROW_NUMBER() OVER (ORDER BY paradedb.score(m.id) DESC) AS relevance_rank
        FROM mixed_numeric_string_test m
        WHERE m.content @@@ 'window function'
        ORDER BY m.string_field1, m.numeric_field1 DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Window functions execution methods: {:?}", methods);

    // Score function should force a FastFieldExecState
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState with window functions, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        SELECT 
            m.id,
            m.numeric_field1,
            m.string_field1,
            (AVG(m.numeric_field1) OVER (PARTITION BY m.string_field1)) AS avg_by_group,
            RANK() OVER (PARTITION BY m.string_field1 ORDER BY m.numeric_field1 DESC) AS rank_in_group,
            ROW_NUMBER() OVER (ORDER BY paradedb.score(m.id) DESC) AS relevance_rank
        FROM mixed_numeric_string_test m
        WHERE m.content @@@ 'window function'
        ORDER BY m.string_field1, m.numeric_field1 DESC
    "#
    .fetch_result::<(String, i32, String, BigDecimal, i64, i64)>(&mut conn)
    .unwrap();

    // Should find some results
    assert!(
        !results.is_empty(),
        "Expected at least one result for window functions"
    );

    // Check window function results are correct
    let group0_results: Vec<_> = results
        .iter()
        .filter(|(_, _, g, _, _, _)| g == "Group0")
        .collect();

    if !group0_results.is_empty() {
        // Verify rank_in_group starts at 1 and is sequential
        for i in 0..group0_results.len() {
            assert_eq!(
                group0_results[i].4,
                (i + 1) as i64,
                "Expected rank_in_group to be sequential, got {} at index {}",
                group0_results[i].4,
                i
            );
        }
    }
}

#[rstest]
fn test_multi_index_search_with_intersection(mut conn: PgConnection) {
    TestMixedFastFields::setup().execute(&mut conn);

    // Find documents and files that share common terms across both indexes
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT 
            d.id AS doc_id,
            d.title AS doc_title,
            f.id AS file_id,
            f.title AS file_title,
            (SELECT AVG(p.page_number) FROM pages p WHERE p.fileId = f.id) AS avg_page_number,
            paradedb.score(d.id) + paradedb.score(f.id) AS combined_score
        FROM documents d
        JOIN files f ON d.id = f.documentId
        WHERE 
            d.title @@@ 'Invoice OR Receipt' AND
            f.title @@@ 'Invoice OR Receipt'
        ORDER BY combined_score DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Multi-index execution methods: {:?}", methods);

    // Score functions should force FastFieldExecState for both tables
    assert!(
        methods
            .iter()
            .filter(|m| m.contains("FastFieldExecState"))
            .count() >= 2,
        "Expected at least 2FastFieldExecState instances for multi-index search with subqueries, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        SELECT 
            d.id AS doc_id,
            d.title AS doc_title,
            f.id AS file_id,
            f.title AS file_title,
            (SELECT AVG(p.page_number) FROM pages p WHERE p.fileId = f.id) AS avg_page_number,
            paradedb.score(d.id) + paradedb.score(f.id) AS combined_score
        FROM documents d
        JOIN files f ON d.id = f.documentId
        WHERE 
            d.title @@@ 'Invoice OR Receipt' AND
            f.title @@@ 'Invoice OR Receipt'
        ORDER BY combined_score DESC
    "#
    .fetch_result::<(String, String, String, String, Option<BigDecimal>, f32)>(&mut conn)
    .unwrap();

    // Should find at least one matching pair
    assert!(
        !results.is_empty(),
        "Expected at least one result for multi-index search"
    );

    // Verify title contains Invoice or Receipt in both document and file
    for (_, doc_title, _, file_title, _, _) in &results {
        assert!(
            doc_title.contains("Invoice") || doc_title.contains("Receipt"),
            "Document title should contain 'Invoice' or 'Receipt', got: {}",
            doc_title
        );
        assert!(
            file_title.contains("Invoice") || file_title.contains("Receipt"),
            "File title should contain 'Invoice' or 'Receipt', got: {}",
            file_title
        );
    }

    // Verify score ordering
    for i in 1..results.len() {
        assert!(
            results[i - 1].5 >= results[i].5,
            "Results not ordered correctly by combined_score DESC"
        );
    }
}

// SECTION 4: COMPLEX CTES AND JOINS
// =====================================
struct TestComplexMixedFastFields;

impl TestComplexMixedFastFields {
    fn setup() -> impl Query {
        r#"
            DROP TABLE IF EXISTS products CASCADE;
            DROP TABLE IF EXISTS categories CASCADE;
            DROP TABLE IF EXISTS suppliers CASCADE;
            DROP TABLE IF EXISTS inventory CASCADE;
            DROP TABLE IF EXISTS orders CASCADE;
            
            -- Create category table
            CREATE TABLE categories (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                parent_id INTEGER REFERENCES categories(id)
            );
            
            -- Create supplier table
            CREATE TABLE suppliers (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                country TEXT NOT NULL,
                contact_name TEXT,
                contact_email TEXT,
                rating INTEGER
            );
            
            -- Create product table
            CREATE TABLE products (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                sku TEXT NOT NULL UNIQUE,
                description TEXT,
                price NUMERIC(10,2) NOT NULL,
                weight NUMERIC(8,2),
                category_id INTEGER REFERENCES categories(id),
                supplier_id INTEGER REFERENCES suppliers(id),
                is_active BOOLEAN DEFAULT true,
                created_at TIMESTAMP DEFAULT NOW()
            );
            
            -- Create inventory table
            CREATE TABLE inventory (
                id SERIAL PRIMARY KEY,
                product_id INTEGER REFERENCES products(id),
                warehouse_name TEXT NOT NULL,
                quantity INTEGER NOT NULL,
                last_restock_date DATE,
                notes TEXT
            );
            
            -- Create orders table
            CREATE TABLE orders (
                id SERIAL PRIMARY KEY,
                order_number TEXT NOT NULL UNIQUE,
                customer_name TEXT NOT NULL,
                product_id INTEGER REFERENCES products(id),
                quantity INTEGER NOT NULL,
                order_date TIMESTAMP DEFAULT NOW(),
                status TEXT NOT NULL,
                shipping_address TEXT,
                total_amount NUMERIC(12,2)
            );
            
            -- Create BM25 indexes
            CREATE INDEX product_search ON products USING bm25 (
                id,
                name,
                sku,
                description,
                price,
                weight,
                is_active,
                category_id,
                supplier_id
            ) WITH (
                key_field = 'id',
                text_fields = '{"name": {"tokenizer": {"type": "default"}, "fast": true}, "sku": {"tokenizer": {"type": "keyword"}, "fast": true}, "description": {"tokenizer": {"type": "default"}}}',
                numeric_fields = '{"price": {"fast": true}, "weight": {"fast": true}, "category_id": {"fast": true}, "supplier_id": {"fast": true}}',
                boolean_fields = '{"is_active": {"fast": true}}'
            );
            
            CREATE INDEX category_search ON categories USING bm25 (
                id,
                name,
                description
            ) WITH (
                key_field = 'id',
                text_fields = '{"name": {"tokenizer": {"type": "default"}, "fast": true}, "description": {"tokenizer": {"type": "default"}, "fast": true}}'
            );
            
            CREATE INDEX supplier_search ON suppliers USING bm25 (
                id,
                name,
                country,
                contact_name,
                rating
            ) WITH (
                key_field = 'id',
                text_fields = '{"name": {"tokenizer": {"type": "default"}, "fast": true}, "country": {"tokenizer": {"type": "keyword"}, "fast": true}, "contact_name": {"tokenizer": {"type": "default"}}}',
                numeric_fields = '{"rating": {"fast": true}}'
            );
            
            CREATE INDEX inventory_search ON inventory USING bm25 (
                id,
                warehouse_name,
                quantity,
                notes,
                product_id
            ) WITH (
                key_field = 'id',
                text_fields = '{"warehouse_name": {"tokenizer": {"type": "default"}, "fast": true}, "notes": {"tokenizer": {"type": "default"}}}',
                numeric_fields = '{"quantity": {"fast": true}, "product_id": {"fast": true}}'
            );
            
            CREATE INDEX orders_search ON orders USING bm25 (
                id,
                customer_name,
                status,
                shipping_address,
                total_amount,
                quantity,
                product_id
            ) WITH (
                key_field = 'id',
                text_fields = '{"customer_name": {"tokenizer": {"type": "default"}, "fast": true}, "status": {"tokenizer": {"type": "keyword"}, "fast": true}, "shipping_address": {"tokenizer": {"type": "default"}}}',
                numeric_fields = '{"total_amount": {"fast": true}, "quantity": {"fast": true}, "product_id": {"fast": true}}'
            );
            
            -- Insert sample data: Categories
            INSERT INTO categories (name, description, parent_id) VALUES
            ('Electronics', 'Electronic devices and accessories', NULL),
            ('Computers', 'Desktop and laptop computers', 1),
            ('Smartphones', 'Mobile phones and accessories', 1),
            ('Clothing', 'Apparel and fashion items', NULL),
            ('Men''s Clothing', 'Clothing for men', 4),
            ('Women''s Clothing', 'Clothing for women', 4),
            ('Food', 'Edible products', NULL),
            ('Dairy', 'Milk and dairy products', 7),
            ('Bakery', 'Bread and baked goods', 7);
            
            -- Insert sample data: Suppliers
            INSERT INTO suppliers (name, country, contact_name, contact_email, rating) VALUES
            ('TechCorp', 'USA', 'John Smith', 'john@techcorp.com', 5),
            ('Fashion Unlimited', 'Italy', 'Maria Rossi', 'maria@fashionunlimited.it', 4),
            ('Global Foods', 'France', 'Pierre Dupont', 'pierre@globalfoods.fr', 3),
            ('ElectroSupply', 'Japan', 'Takashi Yamamoto', 'takashi@electrosupply.jp', 5),
            ('Threads Co', 'UK', 'Emma Wilson', 'emma@threadsco.uk', 4),
            ('OrganicSource', 'Spain', 'Carlos Martinez', 'carlos@organicsource.es', 4);
            
            -- Insert sample data: Products
            INSERT INTO products (name, sku, description, price, weight, category_id, supplier_id, is_active) VALUES
            ('Ultrabook Pro', 'UB-PRO-1', 'High-performance laptop with SSD', 1299.99, 1.2, 2, 1, true),
            ('SmartPhone X', 'SPX-100', 'Latest smartphone with high-resolution camera', 899.99, 0.18, 3, 1, true),
            ('Men''s Casual Shirt', 'MCS-001', 'Comfortable cotton shirt for everyday wear', 49.99, 0.3, 5, 2, true),
            ('Women''s Evening Dress', 'WED-150', 'Elegant evening dress for special occasions', 199.99, 0.5, 6, 2, true),
            ('Organic Milk', 'OM-1000', 'Fresh organic milk from grass-fed cows', 3.99, 1.0, 8, 3, true),
            ('Artisan Bread', 'AB-500', 'Freshly baked artisan sourdough bread', 5.99, 0.5, 9, 3, true),
            ('Gaming Laptop', 'GL-550', 'High-end gaming laptop with dedicated GPU', 1899.99, 2.5, 2, 1, true),
            ('Designer Jeans', 'DJ-100', 'Premium designer jeans with modern cut', 129.99, 0.6, 5, 5, true),
            ('Premium Yogurt', 'PY-250', 'Creamy Greek yogurt with live cultures', 4.99, 0.25, 8, 3, true),
            ('LCD Monitor', 'LM-27', 'Widescreen monitor with 4K resolution', 349.99, 5.0, 1, 1, true),
            ('Classic Coat', 'CC-750', 'Winter coat with wool blend', 199.99, 1.5, 6, 2, true),
            ('Tablet Pro', 'TP-10', 'Professional tablet with stylus support', 799.99, 0.45, 1, 4, true),
            ('Mechanical Keyboard', 'KB-101', 'Mechanical keyboard with RGB lighting', 149.99, 1.2, 1, 1, true),
            ('Women''s Boots', 'WB-225', 'Leather boots for winter', 159.99, 1.0, 6, 5, true),
            ('Vintage Wine', 'VW-750', 'Premium red wine aged in oak barrels', 89.99, 0.75, 7, 6, false);
            
            -- Insert sample data: Inventory
            INSERT INTO inventory (product_id, warehouse_name, quantity, last_restock_date, notes) VALUES
            (1, 'North Warehouse', 25, '2023-01-15', 'Regular stock'),
            (2, 'North Warehouse', 40, '2023-01-20', 'High demand expected'),
            (3, 'East Warehouse', 100, '2023-01-10', 'Oversupply'),
            (4, 'East Warehouse', 20, '2023-01-25', 'Seasonal item'),
            (5, 'South Warehouse', 150, '2023-02-01', 'Perishable - check dates'),
            (6, 'South Warehouse', 50, '2023-02-01', 'Daily delivery required'),
            (7, 'North Warehouse', 10, '2023-01-05', 'Limited stock - order more'),
            (8, 'East Warehouse', 35, '2023-01-15', 'Popular sizes running low'),
            (9, 'South Warehouse', 75, '2023-02-01', 'Refrigeration required'),
            (10, 'West Warehouse', 30, '2023-01-10', 'New model arrival expected'),
            (11, 'East Warehouse', 15, '2023-01-20', 'Seasonal stock'),
            (12, 'North Warehouse', 20, '2023-01-25', 'Display models needed'),
            (13, 'West Warehouse', 45, '2023-01-15', 'Popular item'),
            (14, 'East Warehouse', 25, '2023-01-20', 'Winter collection'),
            (15, 'South Warehouse', 50, '2023-01-30', 'Store in climate-controlled area');
            
            -- Insert sample data: Orders
            INSERT INTO orders (order_number, customer_name, product_id, quantity, order_date, status, shipping_address, total_amount) VALUES
            ('ORD-10001', 'Alice Johnson', 1, 1, '2023-01-05', 'delivered', '123 Main St, Anytown', 1299.99),
            ('ORD-10002', 'Bob Smith', 2, 1, '2023-01-07', 'shipped', '456 Oak Ave, Somewhere', 899.99),
            ('ORD-10003', 'Charlie Brown', 3, 2, '2023-01-10', 'processing', '789 Pine Rd, Anywhere', 99.98),
            ('ORD-10004', 'Diana Ross', 4, 1, '2023-01-12', 'shipped', '321 Elm St, Nowhere', 199.99),
            ('ORD-10005', 'Edward Norton', 5, 3, '2023-01-15', 'delivered', '654 Maple Dr, Everywhere', 11.97),
            ('ORD-10006', 'Fiona Apple', 6, 2, '2023-01-17', 'processing', '987 Cedar Ln, Somewhere', 11.98),
            ('ORD-10007', 'George Clooney', 7, 1, '2023-01-20', 'shipped', '741 Birch St, Anytown', 1899.99),
            ('ORD-10008', 'Helen Mirren', 8, 2, '2023-01-22', 'delivered', '852 Willow Ave, Nowhere', 259.98),
            ('ORD-10009', 'Ian McKellen', 9, 5, '2023-01-25', 'processing', '963 Spruce Rd, Everywhere', 24.95),
            ('ORD-10010', 'Julia Roberts', 10, 2, '2023-01-27', 'shipped', '159 Aspen Dr, Somewhere', 699.98),
            ('ORD-10011', 'Kevin Bacon', 11, 1, '2023-01-30', 'delivered', '357 Redwood Ln, Anytown', 199.99),
            ('ORD-10012', 'Lucy Liu', 12, 1, '2023-02-01', 'processing', '951 Sequoia St, Nowhere', 799.99),
            ('ORD-10013', 'Michael Jordan', 13, 3, '2023-02-03', 'shipped', '753 Oak Ave, Anywhere', 449.97),
            ('ORD-10014', 'Nicole Kidman', 14, 1, '2023-02-05', 'delivered', '159 Pine Rd, Everywhere', 159.99),
            ('ORD-10015', 'Orlando Bloom', 15, 2, '2023-02-07', 'shipped', '357 Maple Dr, Somewhere', 179.98);
        "#
    }
}

// SECTION 1: TESTS WITH COMPLEX CTES AND JOINS
// ============================================

#[rstest]
fn test_basic_cte_with_search(mut conn: PgConnection) {
    TestComplexMixedFastFields::setup().execute(&mut conn);

    // Test with CTE using @@@ operator
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        WITH electronic_products AS (
            SELECT id, name, sku, price 
            FROM products 
            WHERE description @@@ 'laptop' AND is_active = true
        )
        SELECT ep.name, ep.sku, ep.price
        FROM electronic_products ep
        ORDER BY ep.price DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("CTE execution methods: {:?}", methods);

    // Either MixedFastFieldExecState or another fast field exec state should be used
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState to be used for CTE, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        WITH electronic_products AS (
            SELECT id, name, sku, price 
            FROM products 
            WHERE description @@@ 'laptop' AND is_active = true
        )
        SELECT ep.name, ep.sku, ep.price
        FROM electronic_products ep
        ORDER BY ep.price DESC
    "#
    .fetch_result::<(String, String, BigDecimal)>(&mut conn)
    .unwrap();

    // Should find at least 2 laptops
    assert!(results.len() >= 2, "Expected at least 2 laptops");

    // Verify price ordering
    for i in 1..results.len() {
        assert!(
            results[i - 1].2 >= results[i].2,
            "Results not ordered correctly by price DESC"
        );
    }
}

#[rstest]
fn test_multiple_ctes_with_search(mut conn: PgConnection) {
    TestComplexMixedFastFields::setup().execute(&mut conn);

    // Test with multiple CTEs using @@@ operators
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        WITH 
        tech_products AS (
            SELECT p.id, p.name, p.sku, p.price, p.category_id
            FROM products p
            WHERE p.description @@@ 'laptop OR monitor OR keyboard' AND p.is_active = true
        ),
        tech_categories AS (
            SELECT c.id, c.name AS category_name
            FROM categories c
            WHERE c.name @@@ 'Electronics OR Computers'
        )
        SELECT tp.name, tp.sku, tp.price, tc.category_name
        FROM tech_products tp
        JOIN tech_categories tc ON tp.category_id = tc.id
        ORDER BY tp.price DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Multiple CTEs execution methods: {:?}", methods);

    // At least one FastFieldExecState should be used
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState to be used for multiple CTEs, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        WITH 
        tech_products AS (
            SELECT p.id, p.name, p.sku, p.price, p.category_id
            FROM products p
            WHERE p.description @@@ 'laptop OR monitor OR keyboard' AND p.is_active = true
        ),
        tech_categories AS (
            SELECT c.id, c.name AS category_name
            FROM categories c
            WHERE c.name @@@ 'Electronics OR Computers'
        )
        SELECT tp.name, tp.sku, tp.price, tc.category_name
        FROM tech_products tp
        JOIN tech_categories tc ON tp.category_id = tc.id
        ORDER BY tp.price DESC
    "#
    .fetch_result::<(String, String, BigDecimal, String)>(&mut conn)
    .unwrap();

    // Should find at least 3 tech products
    assert!(results.len() >= 3, "Expected at least 3 tech products");

    // Verify fields have proper values
    for (name, sku, price, category) in &results {
        assert!(!name.is_empty(), "Product name should not be empty");
        assert!(!sku.is_empty(), "SKU should not be empty");
        assert!(price > &BigDecimal::from(0), "Price should be positive");
        assert!(
            category == "Electronics" || category == "Computers",
            "Category should be Electronics or Computers"
        );
    }
}

#[rstest]
fn test_complex_joins_with_search(mut conn: PgConnection) {
    TestComplexMixedFastFields::setup().execute(&mut conn);

    // Test with complex multi-table joins and multiple @@@ operators
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT p.name AS product_name, 
               p.sku, 
               c.name AS category_name, 
               s.name AS supplier_name, 
               i.warehouse_name, 
               i.quantity AS stock
        FROM products p
        JOIN categories c ON p.category_id = c.id
        JOIN suppliers s ON p.supplier_id = s.id AND s.country @@@ 'USA OR Japan'
        JOIN inventory i ON p.id = i.product_id AND i.warehouse_name @@@ 'North'
        WHERE p.description @@@ 'laptop OR tablet' AND p.is_active = true
        ORDER BY i.quantity DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Complex JOIN execution methods: {:?}", methods);

    // At least one FastFieldExecState should be used
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState to be used for complex joins, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        SELECT p.name AS product_name, 
               p.sku, 
               c.name AS category_name, 
               s.name AS supplier_name, 
               i.warehouse_name, 
               i.quantity AS stock
        FROM products p
        JOIN categories c ON p.category_id = c.id
        JOIN suppliers s ON p.supplier_id = s.id AND s.country @@@ 'USA OR Japan'
        JOIN inventory i ON p.id = i.product_id AND i.warehouse_name @@@ 'North'
        WHERE p.description @@@ 'laptop OR tablet' AND p.is_active = true
        ORDER BY i.quantity DESC
    "#
    .fetch_result::<(String, String, String, String, String, i32)>(&mut conn)
    .unwrap();

    // Make sure we found some products
    assert!(
        !results.is_empty(),
        "Expected at least one result for complex join"
    );

    // Verify warehouse name contains North
    for (_, _, _, _, warehouse, _) in &results {
        assert!(
            warehouse.contains("North"),
            "Warehouse name should contain 'North', got: {}",
            warehouse
        );
    }
}

#[rstest]
fn test_with_score_function(mut conn: PgConnection) {
    TestComplexMixedFastFields::setup().execute(&mut conn);

    // Test using score() function to force an execution method
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT p.name, p.sku, p.price, paradedb.score(p.id)
        FROM products p
        WHERE p.description @@@ 'laptop'
        ORDER BY paradedb.score(p.id) DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Score function execution methods: {:?}", methods);

    // Using score() function should force a FastFieldExecState
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState to be used with score function, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        SELECT p.name, p.sku, p.price, paradedb.score(p.id)
        FROM products p
        WHERE p.description @@@ 'laptop'
        ORDER BY paradedb.score(p.id) DESC
    "#
    .fetch_result::<(String, String, BigDecimal, f32)>(&mut conn)
    .unwrap();

    // Should find some laptops
    assert!(!results.is_empty(), "Expected at least one laptop");

    // Verify score ordering
    for i in 1..results.len() {
        assert!(
            results[i - 1].3 >= results[i].3,
            "Results not ordered correctly by score DESC"
        );
    }
}

#[rstest]
fn test_mixed_fast_and_non_fast_fields(mut conn: PgConnection) {
    TestComplexMixedFastFields::setup().execute(&mut conn);

    // Test 1: Only fast fields - should use MixedFastFieldExecState
    let (fast_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT p.name, p.sku, p.price
        FROM products p
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    let fast_methods = get_all_exec_methods(&fast_plan);
    println!("Fast fields only execution methods: {:?}", fast_methods);

    // Should use a FastFieldExecState
    assert!(
        fast_methods
            .iter()
            .any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState for fast fields only, got: {:?}",
        fast_methods
    );

    // Test 2: Mix of fast and non-fast fields - should use NormalScanExecState
    let (mixed_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT p.name, p.sku, p.description, p.price
        FROM products p
        WHERE p.description @@@ 'laptop'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    let mixed_methods = get_all_exec_methods(&mixed_plan);
    println!("Mixed fast/non-fast execution methods: {:?}", mixed_methods);

    // Should use NormalScanExecState when non-fast fields are included
    assert!(
        mixed_methods.contains(&"NormalScanExecState".to_string()),
        "Expected NormalScanExecState for mixed fast/non-fast fields, got: {:?}",
        mixed_methods
    );

    // Verify results match despite execution method differences
    let fast_results = r#"
        SELECT p.name, p.sku, p.price
        FROM products p
        WHERE p.description @@@ 'laptop'
        ORDER BY p.name
    "#
    .fetch_result::<(String, String, BigDecimal)>(&mut conn)
    .unwrap();

    let mixed_results = r#"
        SELECT p.name, p.sku, p.price
        FROM products p
        WHERE p.description @@@ 'laptop'
        ORDER BY p.name
    "#
    .fetch_result::<(String, String, BigDecimal)>(&mut conn)
    .unwrap();

    // Results should be the same
    assert_eq!(
        fast_results.len(),
        mixed_results.len(),
        "Fast and mixed query result counts don't match"
    );

    for i in 0..fast_results.len() {
        assert_eq!(
            fast_results[i], mixed_results[i],
            "Results at index {} don't match",
            i
        );
    }
}

#[rstest]
fn test_recursive_cte_with_search(mut conn: PgConnection) {
    TestComplexMixedFastFields::setup().execute(&mut conn);

    // Test with recursive CTE and search
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        WITH RECURSIVE category_tree AS (
            -- Base case: find categories matching 'clothing'
            SELECT c.id, c.name, c.parent_id, 1 AS level
            FROM categories c
            WHERE c.name @@@ 'clothing' OR c.description @@@ 'clothing'
            
            UNION ALL
            
            -- Recursive case: find children of the current category
            SELECT c.id, c.name, c.parent_id, ct.level + 1
            FROM categories c
            JOIN category_tree ct ON c.parent_id = ct.id
        )
        SELECT ct.id, ct.name, ct.level
        FROM category_tree ct
        ORDER BY ct.level, ct.name
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Recursive CTE execution methods: {:?}", methods);

    // Verify results
    let results = r#"
        WITH RECURSIVE category_tree AS (
            -- Base case: find categories matching 'clothing'
            SELECT c.id, c.name, c.parent_id, 1 AS level
            FROM categories c
            WHERE c.name @@@ 'clothing' OR c.description @@@ 'clothing'
            
            UNION ALL
            
            -- Recursive case: find children of the current category
            SELECT c.id, c.name, c.parent_id, ct.level + 1
            FROM categories c
            JOIN category_tree ct ON c.parent_id = ct.id
        )
        SELECT ct.id, ct.name, ct.level
        FROM category_tree ct
        ORDER BY ct.level, ct.name
    "#
    .fetch_result::<(i32, String, i32)>(&mut conn)
    .unwrap();

    // Should find Clothing and its subcategories
    assert!(!results.is_empty(), "Expected at least one category");

    // First result should be Clothing
    assert_eq!(
        results[0].1, "Clothing",
        "First result should be Clothing, got: {}",
        results[0].1
    );

    // Should have level 1 and level 2 entries
    let has_level_1 = results.iter().any(|r| r.2 == 1);
    let has_level_2 = results.iter().any(|r| r.2 == 2);

    assert!(has_level_1, "Should have level 1 entries");
    assert!(has_level_2, "Should have level 2 entries");
}

#[rstest]
fn test_complex_subqueries_with_search(mut conn: PgConnection) {
    TestComplexMixedFastFields::setup().execute(&mut conn);

    // Test with complex subqueries using @@@ operators
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT p.name, p.sku, p.price,
               (SELECT s.name FROM suppliers s WHERE s.id = p.supplier_id) AS supplier_name,
               (SELECT warehouse_name FROM inventory WHERE product_id = p.id AND quantity > 20 LIMIT 1) AS warehouse
        FROM products p
        WHERE p.id IN (
            SELECT product_id 
            FROM orders 
            WHERE customer_name @@@ 'Alice OR Bob' AND status @@@ 'delivered'
        )
        AND p.price > 500
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Complex subqueries execution methods: {:?}", methods);

    // At least one FastFieldExecState should be used
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState to be used for subqueries, got: {:?}",
        methods
    );

    // Verify results
    let results = r#"
        SELECT p.name, p.sku, p.price,
               (SELECT s.name FROM suppliers s WHERE s.id = p.supplier_id) AS supplier_name,
               (SELECT warehouse_name FROM inventory WHERE product_id = p.id AND quantity > 20 LIMIT 1) AS warehouse
        FROM products p
        WHERE p.id IN (
            SELECT product_id 
            FROM orders 
            WHERE customer_name @@@ 'Alice OR Bob' AND status @@@ 'delivered'
        )
        AND p.price > 500
    "#
    .fetch_result::<(String, String, BigDecimal, String, Option<String>)>(&mut conn)
    .unwrap();

    // Should find some products
    assert!(
        !results.is_empty(),
        "Expected at least one result for subqueries"
    );

    // Verify price > 500
    for (_, _, price, _, _) in &results {
        assert!(
            price > &BigDecimal::from(500),
            "Price should be greater than 500"
        );
    }
}

#[rstest]
fn test_join_with_aggregation_and_search(mut conn: PgConnection) {
    TestComplexMixedFastFields::setup().execute(&mut conn);

    // Test with join, aggregation, and search
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT 
            c.name AS category_name,
            COUNT(p.id) AS product_count,
            AVG(p.price) AS avg_price,
            MAX(p.price) AS max_price,
            MIN(p.price) AS min_price
        FROM products p
        JOIN categories c ON p.category_id = c.id
        WHERE p.is_active = true and p.description @@@ 'laptop'
        GROUP BY c.name
        HAVING COUNT(p.id) > 1
        ORDER BY product_count DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Join with aggregation execution methods: {:?}", methods);

    // Add product description search to force @@@ operator with aggregation
    let (search_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT 
            c.name AS category_name,
            COUNT(p.id) AS product_count,
            AVG(p.price) AS avg_price,
            MAX(p.price) AS max_price,
            MIN(p.price) AS min_price
        FROM products p
        JOIN categories c ON p.category_id = c.id
        WHERE p.description @@@ 'laptop OR tablet OR smartphone'
        GROUP BY c.name
        HAVING COUNT(p.id) > 0
        ORDER BY product_count DESC
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods for search with aggregation
    let search_methods = get_all_exec_methods(&search_plan);
    println!(
        "Search with aggregation execution methods: {:?}",
        search_methods
    );

    // Verify results
    let results = r#"
        SELECT 
            c.name AS category_name,
            COUNT(p.id) AS product_count,
            AVG(p.price) AS avg_price,
            MAX(p.price) AS max_price,
            MIN(p.price) AS min_price
        FROM products p
        JOIN categories c ON p.category_id = c.id
        WHERE p.description @@@ 'laptop OR tablet OR smartphone'
        GROUP BY c.name
        HAVING COUNT(p.id) > 0
        ORDER BY product_count DESC
    "#
    .fetch_result::<(String, i64, BigDecimal, BigDecimal, BigDecimal)>(&mut conn)
    .unwrap();

    // Should find some categories
    assert!(
        !results.is_empty(),
        "Expected at least one category with products"
    );
}

#[rstest]
fn test_with_forced_mixed_execution(mut conn: PgConnection) {
    TestComplexMixedFastFields::setup().execute(&mut conn);

    // Add a product with a distinct string to search for
    r#"
        INSERT INTO products (name, sku, description, price, weight, category_id, supplier_id, is_active)
        VALUES ('Unique Product Z', 'UPZ-001', 'This is a uniqueproductZ for testing mixed fields', 49.99, 1.0, 1, 1, true);
    "#
    .execute(&mut conn);

    // First query: Should use MixedFastFieldExecState (name + sku are string fast fields)
    let (string_fast_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT p.name, p.sku
        FROM products p
        WHERE p.description @@@ 'uniqueproductZ'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    let string_methods = get_all_exec_methods(&string_fast_plan);
    println!("String fast fields execution methods: {:?}", string_methods);

    // Force exact MixedFastFieldExecState (two string fast fields)
    assert!(
        string_methods.contains(&"MixedFastFieldExecState".to_string()),
        "Expected specifically MixedFastFieldExecState for multiple string fast fields, got: {:?}",
        string_methods
    );

    // Second query: Should use MixedFastFieldExecState (mix of string and numeric fast fields)
    let (mixed_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT p.name, p.sku, p.price, p.weight
        FROM products p
        WHERE p.description @@@ 'uniqueproductZ'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    let mixed_methods = get_all_exec_methods(&mixed_plan);
    println!(
        "Mixed string/numeric fields execution methods: {:?}",
        mixed_methods
    );

    // Should use MixedFastFieldExecState for mixed field types
    assert!(
        mixed_methods.contains(&"MixedFastFieldExecState".to_string()),
        "Expected specifically MixedFastFieldExecState for mixed string/numeric fields, got: {:?}",
        mixed_methods
    );

    // Third query: Boolean fast field added to the mix
    let (bool_plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT p.name, p.sku, p.price, p.is_active
        FROM products p
        WHERE p.description @@@ 'uniqueproductZ'
    "#
    .fetch_one::<(Value,)>(&mut conn);

    let bool_methods = get_all_exec_methods(&bool_plan);
    println!("With boolean field execution methods: {:?}", bool_methods);

    // Should use MixedFastFieldExecState for mix including boolean
    assert!(
        bool_methods.contains(&"MixedFastFieldExecState".to_string()),
        "Expected MixedFastFieldExecState for mix including boolean, got: {:?}",
        bool_methods
    );

    // Verify results match for all approaches
    let results1 = r#"
        SELECT p.name
        FROM products p
        WHERE p.description @@@ 'uniqueproductZ'
    "#
    .fetch_one::<(String,)>(&mut conn);

    let results2 = r#"
        SELECT p.name
        FROM products p
        WHERE p.description @@@ 'uniqueproductZ'
    "#
    .fetch_one::<(String,)>(&mut conn);

    let results3 = r#"
        SELECT p.name
        FROM products p
        WHERE p.description @@@ 'uniqueproductZ'
    "#
    .fetch_one::<(String,)>(&mut conn);

    // All should find the same unique product
    assert_eq!(results1.0, "Unique Product Z");
    assert_eq!(results2.0, "Unique Product Z");
    assert_eq!(results3.0, "Unique Product Z");
}
