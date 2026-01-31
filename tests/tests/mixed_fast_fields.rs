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

// Setup for complex aggregation with mixed fast fields
fn complex_aggregation_setup() -> &'static str {
    r#"
        DROP TABLE IF EXISTS expected_payments;
        CREATE TABLE expected_payments (
          id                  SERIAL PRIMARY KEY,
          organization_id     UUID     NOT NULL,
          live_mode           BOOLEAN  NOT NULL,
          status              TEXT     NOT NULL,
          internal_account_id UUID     NOT NULL,
          amount_range        NUMRANGE NOT NULL,
          amount_reconciled   NUMERIC  NOT NULL,
          direction           TEXT     NOT NULL CHECK (direction IN ('credit','debit')),
          currency            TEXT     NOT NULL,
          discarded_at        TIMESTAMP NULL
        );

        INSERT INTO expected_payments (
          organization_id,
          live_mode,
          status,
          internal_account_id,
          amount_range,
          amount_reconciled,
          direction,
          currency,
          discarded_at
        )
        SELECT
          organization_id,
          live_mode,
          status,
          internal_account_id,
          numrange(lower_val, lower_val + offset_val)         AS amount_range,
          amount_reconciled,
          direction,
          currency,
          discarded_at
        FROM (
          SELECT
            -- random UUID
            (md5(random()::text))::uuid                        AS organization_id,
            -- 50/50 live_mode
            (random() < 0.5)                                    AS live_mode,
            -- status pick
            (ARRAY['unreconciled','partially_reconciled'])
              [floor(random()*2 + 1)::int]                      AS status,
            -- another random UUID
            (md5(random()::text))::uuid                        AS internal_account_id,
            -- ensure lower ≤ upper by generating an offset
            floor(random()*1000)::int                           AS lower_val,
            floor(random()*100)::int + 1                        AS offset_val,
            -- reconciled amount between –500 and +500
            (random()*1000 - 500)::numeric                      AS amount_reconciled,
            -- direction pick
            (ARRAY['credit','debit'])[floor(random()*2 + 1)::int] AS direction,
            -- currency pick
            (ARRAY['USD','EUR','GBP','JPY','AUD'])[floor(random()*5 + 1)::int] AS currency,
            -- 10% NULL, else random timestamp in last year
            CASE
              WHEN random() < 0.10 THEN NULL
              ELSE now() - (random() * INTERVAL '365 days')
            END                                                 AS discarded_at
          FROM generate_series(1, 1000)
        ) sub;

        create index expected_payments_idx on expected_payments using bm25 (
            id,
            organization_id,
            live_mode,
            status,
            internal_account_id,
            amount_range,
            amount_reconciled,
            direction,
            currency,
            discarded_at
        ) with (
            key_field = 'id',
            text_fields = '{"organization_id": {"fast":true}, "status": {"fast": true, "tokenizer": {"type": "keyword"}}, "direction": {"fast": true}, "currency": {"fast": true}}',
            boolean_fields = '{"live_mode": {"fast": true}}'
        );
    "#
}

#[ignore]
#[rstest]
fn test_complex_aggregation_with_mixed_fast_fields(mut conn: PgConnection) {
    complex_aggregation_setup().execute(&mut conn);

    // Force disable regular index scans to ensure BM25 index is used
    "SET enable_indexscan = off;".execute(&mut conn);

    // Get execution plan for the complex query
    let (plan,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT
          COALESCE(SUM(case when expected_payments.direction = 'credit' then lower(expected_payments.amount_range) else -(upper(expected_payments.amount_range) - 1) end), 0) - COALESCE(SUM(amount_reconciled), 0) total_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then (upper(expected_payments.amount_range) - 1) else -lower(expected_payments.amount_range) end), 0) - COALESCE(SUM(amount_reconciled), 0) total_max_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then lower(expected_payments.amount_range) else 0 end), 0) - SUM(GREATEST(amount_reconciled, 0)) credit_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then (upper(expected_payments.amount_range) - 1) else 0 end), 0) - SUM(GREATEST(amount_reconciled, 0)) credit_max_range, 
          COALESCE(SUM(case when expected_payments.direction = 'debit' then -(upper(expected_payments.amount_range) - 1) else 0 end), 0) - SUM(LEAST(amount_reconciled, 0)) debit_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'debit' then -lower(expected_payments.amount_range) else 0 end), 0) - SUM(LEAST(amount_reconciled, 0)) debit_max_range, 
          COUNT(case when expected_payments.direction = 'credit' then 1 else null end) as credit_count, 
          COUNT(case when expected_payments.direction = 'debit' then 1 else null end) as debit_count, 
          COUNT(*) as total_count, 
          COUNT(distinct expected_payments.currency) as currency_count, 
          (ARRAY_AGG(distinct expected_payments.currency))[1] as currency 
        FROM expected_payments
        WHERE expected_payments.live_mode @@@ 'true' 
          AND expected_payments.status @@@ 'IN [unreconciled partially_reconciled]' 
          AND expected_payments.discarded_at IS NULL 
        LIMIT 1
    "#
    .fetch_one::<(Value,)>(&mut conn);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    println!("Complex aggregation execution methods: {methods:?}");

    // Assert that a fast field execution state is used
    assert!(
        methods.iter().any(|m| m.contains("FastFieldExecState")),
        "Expected a FastFieldExecState to be used for complex aggregation, got: {methods:?}"
    );

    // Actually execute the query to verify results
    let results = r#"
        SELECT
          COALESCE(SUM(case when expected_payments.direction = 'credit' then lower(expected_payments.amount_range) else -(upper(expected_payments.amount_range) - 1) end), 0) - COALESCE(SUM(amount_reconciled), 0) total_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then (upper(expected_payments.amount_range) - 1) else -lower(expected_payments.amount_range) end), 0) - COALESCE(SUM(amount_reconciled), 0) total_max_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then lower(expected_payments.amount_range) else 0 end), 0) - SUM(GREATEST(amount_reconciled, 0)) credit_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'credit' then (upper(expected_payments.amount_range) - 1) else 0 end), 0) - SUM(GREATEST(amount_reconciled, 0)) credit_max_range, 
          COALESCE(SUM(case when expected_payments.direction = 'debit' then -(upper(expected_payments.amount_range) - 1) else 0 end), 0) - SUM(LEAST(amount_reconciled, 0)) debit_min_range, 
          COALESCE(SUM(case when expected_payments.direction = 'debit' then -lower(expected_payments.amount_range) else 0 end), 0) - SUM(LEAST(amount_reconciled, 0)) debit_max_range, 
          COUNT(case when expected_payments.direction = 'credit' then 1 else null end) as credit_count, 
          COUNT(case when expected_payments.direction = 'debit' then 1 else null end) as debit_count, 
          COUNT(*) as total_count, 
          COUNT(distinct expected_payments.currency) as currency_count, 
          (ARRAY_AGG(distinct expected_payments.currency))[1] as currency 
        FROM expected_payments
        WHERE expected_payments.live_mode @@@ 'true' 
          AND expected_payments.status @@@ 'IN [unreconciled partially_reconciled]' 
          AND expected_payments.discarded_at IS NULL 
        LIMIT 1
    "#
    .fetch_result::<(
        BigDecimal,
        BigDecimal,
        BigDecimal,
        BigDecimal,
        BigDecimal,
        BigDecimal,
        i64,
        i64,
        i64,
        i64,
        Option<String>,
    )>(&mut conn)
    .unwrap();

    // Assert that we got results (should be at least one row)
    assert!(!results.is_empty(), "Expected at least one row of results");

    // Get the counts from first result
    let (_, _, _, _, _, _, credit_count, debit_count, total_count, currency_count, currency) =
        &results[0];

    // Verify consistency in counts
    assert_eq!(
        *total_count,
        credit_count + debit_count,
        "Total count should equal credit_count + debit_count"
    );

    // Verify currency count is positive
    assert!(
        *currency_count > 0,
        "Should have at least one currency type"
    );

    // Check that we have a currency value if currency_count > 0
    if *currency_count > 0 {
        assert!(
            currency.is_some(),
            "Should have a currency value when currency_count > 0"
        );
    }

    // Reset setting
    "SET enable_indexscan = on;".execute(&mut conn);
}

fn fast_fields_setup() -> &'static str {
    r#"
        DROP TABLE IF EXISTS mixed_ff_v2;
        CREATE TABLE mixed_ff_v2 (
          id SERIAL PRIMARY KEY,
          title TEXT,
          category TEXT,
          rating INT,
          description TEXT,
          content TEXT,
          price NUMERIC,
          tags TEXT[]
        );

        INSERT INTO mixed_ff_v2 (title, category, rating, description, content, price, tags)
        SELECT
            'Title ' || i,
            'Category ' || (i % 5),
            i,
            'Description ' || i,
            'Content ' || i,
            (i * 1.5)::numeric,
            ARRAY['tag' || (i % 3), 'tag' || (i % 5)]
        FROM generate_series(1, 100) i;

        CREATE INDEX mixed_ff_v2_idx ON mixed_ff_v2 USING bm25 (
            id,
            title,
            content,
            price,
            tags,
            (category::pdb.literal('alias=cat_lit')),
            (rating::pdb.alias('rating_alias')),
            (description::pdb.literal),
            ((title || ' ' || category)::pdb.literal('alias=concat_expr')),
            ((rating + 1)::pdb.alias('rating_plus_one'))
        ) WITH (key_field = 'id');
    "#
}

#[rstest]
#[case::aliased_literal(r#"SELECT category FROM mixed_ff_v2 WHERE title @@@ 'Title'"#, true)]
#[case::unaliased_literal(r#"SELECT description FROM mixed_ff_v2 WHERE title @@@ 'Title'"#, true)]
#[case::simple_expression_id(r#"SELECT (id) FROM mixed_ff_v2 WHERE title @@@ 'Title'"#, true)]
#[case::aliased_integer(r#"SELECT rating FROM mixed_ff_v2 WHERE title @@@ 'Title'"#, true)]
#[case::output_cast(
    r#"SELECT rating::text FROM mixed_ff_v2 WHERE title @@@ 'Title'"#,
    false
)]
#[case::default_tokenizer(r#"SELECT content FROM mixed_ff_v2 WHERE title @@@ 'Title'"#, false)]
#[case::expression_mismatch(
    r#"SELECT lower(title) FROM mixed_ff_v2 WHERE title @@@ 'Title'"#,
    false
)]
#[case::expression_concat(
    r#"SELECT title || ' ' || category FROM mixed_ff_v2 WHERE title @@@ 'Title'"#,
    true
)]
#[case::expression_arithmetic(
    r#"SELECT rating + 1 FROM mixed_ff_v2 WHERE title @@@ 'Title'"#,
    true
)]
#[case::numeric_column(r#"SELECT price FROM mixed_ff_v2 WHERE title @@@ 'Title'"#, false)]
#[case::array_column(r#"SELECT tags FROM mixed_ff_v2 WHERE title @@@ 'Title'"#, false)]
fn test_fast_fields_cases(
    mut conn: PgConnection,
    #[case] query: &str,
    #[case] expect_fast_field: bool,
) {
    fast_fields_setup().execute(&mut conn);
    "SET enable_indexscan = off;".execute(&mut conn);
    "SET paradedb.enable_aggregate_custom_scan = on;".execute(&mut conn);
    "SET paradedb.enable_mixed_fast_field_exec = on;".execute(&mut conn);
    "SET paradedb.mixed_fast_field_exec_column_threshold = 10;".execute(&mut conn);

    let explain_query = format!("EXPLAIN (ANALYZE, FORMAT JSON) {}", query);
    let (plan,) = explain_query.fetch_one::<(Value,)>(&mut conn);
    let methods = get_all_exec_methods(&plan);

    let has_fast_field = methods.iter().any(|m| m.contains("FastFieldExecState"));

    assert_eq!(
        has_fast_field, expect_fast_field,
        "FastField usage mismatch for query: '{}'. Methods: {:?}",
        query, methods
    );

    // Execute query to check match count (should be non-empty)
    let rows = query.to_string().fetch_dynamic(&mut conn);
    assert!(!rows.is_empty(), "Query should return results: {}", query);
}

// ============================================================================
// Sorted Path Tests for MixedFastFieldExecState
// ============================================================================
//
// These tests verify that MixedFastFieldExecState correctly handles the sorted
// path using SortPreservingMergeExec when the index has a sort_by configuration
// AND the query includes an ORDER BY clause matching the sort_by.
//
// Note: The sorted path is only activated when ORDER BY matches sort_by.

/// Helper to check if results are sorted in descending order
fn is_sorted_desc<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|w| w[0] >= w[1])
}

/// Helper to check if results are sorted in ascending order
fn is_sorted_asc<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|w| w[0] <= w[1])
}

/// Test MixedFastFieldExecState with sorted scan (ScanStrategy::Sorted).
///
/// When the index has sort_by configured, mixed fast field exec is enabled,
/// and ORDER BY matches the sort_by, the sorted path should use
/// SortPreservingMergeExec to merge sorted segment outputs into globally sorted results.
#[rstest]
fn mixed_fast_fields_sorted_scan(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);
    "SET paradedb.enable_mixed_fast_field_exec TO true;".execute(&mut conn);

    r#"
        CREATE TABLE test_mff_sorted (
            id SERIAL PRIMARY KEY,
            name TEXT,
            category TEXT,
            score INTEGER
        );

        CREATE INDEX test_mff_sorted_idx ON test_mff_sorted
        USING bm25 (id, name, category, score)
        WITH (
            key_field = 'id',
            text_fields = '{"name": {}, "category": {"fast": true, "tokenizer": {"type": "keyword"}}}',
            numeric_fields = '{"score": {"fast": true}}',
            sort_by = 'score DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert multiple batches to create segments
    for batch in 1..=4 {
        let sql = format!(
            r#"
            INSERT INTO test_mff_sorted (name, category, score)
            SELECT
                'Item ' || i || ' batch{}',
                'Category' || (i % 3),
                (random() * 100)::integer
            FROM generate_series(1, 30) AS i;
            "#,
            batch
        );
        sql.execute(&mut conn);
    }

    // Query selecting multiple fast fields with ORDER BY (triggers sorted path)
    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT name, category, score FROM test_mff_sorted
        WHERE name @@@ 'Item'
        ORDER BY score DESC
    "#
    .fetch_one(&mut conn);

    eprintln!("Mixed fast fields sorted plan: {:#?}", plan);

    // Get execution methods
    let methods = get_all_exec_methods(&plan);
    eprintln!("Execution methods: {:?}", methods);

    // Query with ORDER BY and verify results are sorted
    let results: Vec<(String, String, i32)> = r#"
        SELECT name, category, score FROM test_mff_sorted
        WHERE name @@@ 'Item'
        ORDER BY score DESC
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 120, "Should return all 120 results");

    let scores: Vec<i32> = results.iter().map(|(_, _, s)| *s).collect();
    assert!(
        is_sorted_desc(&scores),
        "Results should be sorted by score DESC. First 20 scores: {:?}",
        &scores[..20.min(scores.len())]
    );
}

/// Test MixedFastFieldExecState sorted path with parallel workers.
///
/// Verifies that the sorted path works correctly with parallel execution,
/// where each worker claims segments via lazy checkout and produces sorted
/// output that Gather Merge combines. Requires ORDER BY to trigger the sorted path.
#[rstest]
fn mixed_fast_fields_sorted_parallel(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 17 {
        eprintln!("Skipping test: requires PG17+ for debug_parallel_query");
        return;
    }

    "SET max_parallel_workers TO 4;".execute(&mut conn);
    "SET max_parallel_workers_per_gather TO 4;".execute(&mut conn);
    "SET debug_parallel_query TO on;".execute(&mut conn);
    "SET paradedb.enable_mixed_fast_field_exec TO true;".execute(&mut conn);

    r#"
        CREATE TABLE test_mff_parallel (
            id SERIAL PRIMARY KEY,
            title TEXT,
            tag TEXT,
            priority INTEGER
        );

        CREATE INDEX test_mff_parallel_idx ON test_mff_parallel
        USING bm25 (id, title, tag, priority)
        WITH (
            key_field = 'id',
            text_fields = '{"title": {}, "tag": {"fast": true, "tokenizer": {"type": "keyword"}}}',
            numeric_fields = '{"priority": {"fast": true}}',
            sort_by = 'priority DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert enough data to encourage parallel execution
    for batch in 1..=8 {
        let sql = format!(
            r#"
            INSERT INTO test_mff_parallel (title, tag, priority)
            SELECT
                'Document ' || i || ' batch{}',
                'Tag' || (i % 5),
                {} + (random() * 50)::integer
            FROM generate_series(1, 40) AS i;
            "#,
            batch,
            batch * 100 // Different priority ranges per batch
        );
        sql.execute(&mut conn);
    }

    // Query with multiple fast fields and ORDER BY (triggers sorted path)
    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT title, tag, priority FROM test_mff_parallel
        WHERE title @@@ 'Document'
        ORDER BY priority DESC
    "#
    .fetch_one(&mut conn);

    eprintln!("Mixed fast fields parallel sorted plan: {:#?}", plan);

    // Verify results are sorted with ORDER BY
    let results: Vec<(String, String, i32)> = r#"
        SELECT title, tag, priority FROM test_mff_parallel
        WHERE title @@@ 'Document'
        ORDER BY priority DESC
    "#
    .fetch(&mut conn);

    assert_eq!(
        results.len(),
        320,
        "Should return all 320 results (8 batches x 40)"
    );

    let priorities: Vec<i32> = results.iter().map(|(_, _, p)| *p).collect();
    assert!(
        is_sorted_desc(&priorities),
        "Results should be sorted DESC with parallel workers and ORDER BY. First 20: {:?}",
        &priorities[..20.min(priorities.len())]
    );
}

/// Test MixedFastFieldExecState with ASC sort order.
/// Requires ORDER BY to trigger the sorted path.
#[rstest]
fn mixed_fast_fields_sorted_asc(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);
    "SET paradedb.enable_mixed_fast_field_exec TO true;".execute(&mut conn);

    r#"
        CREATE TABLE test_mff_asc (
            id SERIAL PRIMARY KEY,
            content TEXT,
            label TEXT,
            rank INTEGER
        );

        CREATE INDEX test_mff_asc_idx ON test_mff_asc
        USING bm25 (id, content, label, rank)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}, "label": {"fast": true, "tokenizer": {"type": "keyword"}}}',
            numeric_fields = '{"rank": {"fast": true}}',
            sort_by = 'rank ASC NULLS FIRST'
        );
    "#
    .execute(&mut conn);

    // Insert data across multiple segments
    for batch in 1..=4 {
        let sql = format!(
            r#"
            INSERT INTO test_mff_asc (content, label, rank)
            SELECT
                'Entry ' || i || ' batch{}',
                'Label' || (i % 4),
                (random() * 100)::integer
            FROM generate_series(1, 25) AS i;
            "#,
            batch
        );
        sql.execute(&mut conn);
    }

    // Query with ORDER BY and verify ASC ordering
    let results: Vec<(String, String, i32)> = r#"
        SELECT content, label, rank FROM test_mff_asc
        WHERE content @@@ 'Entry'
        ORDER BY rank ASC
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 100, "Should return all 100 results");

    let ranks: Vec<i32> = results.iter().map(|(_, _, r)| *r).collect();
    assert!(
        is_sorted_asc(&ranks),
        "Results should be sorted by rank ASC with ORDER BY. First 20 ranks: {:?}",
        &ranks[..20.min(ranks.len())]
    );
}

/// Test that sorting still works when MixedFastFieldExecState is disabled.
///
/// When enable_mixed_fast_field_exec is false, queries with ORDER BY should
/// still produce correctly sorted results (PostgreSQL will handle sorting).
#[rstest]
fn mixed_fast_fields_disabled_still_works(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);
    "SET paradedb.enable_mixed_fast_field_exec TO false;".execute(&mut conn);

    r#"
        CREATE TABLE test_mff_disabled (
            id SERIAL PRIMARY KEY,
            text_col TEXT,
            str_col TEXT,
            num_col INTEGER
        );

        CREATE INDEX test_mff_disabled_idx ON test_mff_disabled
        USING bm25 (id, text_col, str_col, num_col)
        WITH (
            key_field = 'id',
            text_fields = '{"text_col": {}, "str_col": {"fast": true, "tokenizer": {"type": "keyword"}}}',
            numeric_fields = '{"num_col": {"fast": true}}',
            sort_by = 'num_col DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    for batch in 1..=3 {
        let sql = format!(
            r#"
            INSERT INTO test_mff_disabled (text_col, str_col, num_col)
            SELECT
                'Record ' || i || ' batch{}',
                'Str' || (i % 3),
                (random() * 50)::integer
            FROM generate_series(1, 20) AS i;
            "#,
            batch
        );
        sql.execute(&mut conn);
    }

    // With ORDER BY, PostgreSQL ensures sorted results even with mixed ff exec disabled
    let results: Vec<(i32, i32)> = r#"
        SELECT id, num_col FROM test_mff_disabled
        WHERE text_col @@@ 'Record'
        ORDER BY num_col DESC
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 60, "Should return all 60 results");

    let nums: Vec<i32> = results.iter().map(|(_, n)| *n).collect();
    assert!(
        is_sorted_desc(&nums),
        "Results should be sorted with ORDER BY even when mixed ff exec disabled. First 20: {:?}",
        &nums[..20.min(nums.len())]
    );
}
