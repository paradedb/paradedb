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
struct TestComplexAggregation;

impl TestComplexAggregation {
    fn setup() -> impl Query {
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
}

#[ignore]
#[rstest]
fn test_complex_aggregation_with_mixed_fast_fields(mut conn: PgConnection) {
    TestComplexAggregation::setup().execute(&mut conn);

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
