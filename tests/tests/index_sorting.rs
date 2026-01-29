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

//! Tests for index-level sorting via the `sort_by` CREATE INDEX option.
//!
//! These tests verify that when an index is created with `sort_by`, queries
//! return results in the specified sort order WITHOUT requiring an explicit
//! ORDER BY clause. This is achieved via `SortPreservingMergeExec` which
//! merges sorted segment outputs into a globally sorted result.

mod fixtures;

use fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

/// Helper to check if results are sorted in descending order by a given column
fn is_sorted_desc<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|w| w[0] >= w[1])
}

/// Helper to check if results are sorted in ascending order by a given column
fn is_sorted_asc<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|w| w[0] <= w[1])
}

/// Test that a simple index scan (no ORDER BY) returns sorted results
/// when the index is created with sort_by and has multiple segments.
#[rstest]
fn index_sort_by_desc_multi_segment(mut conn: PgConnection) {
    // Disable parallel workers to ensure predictable behavior
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    // Create table and index with sort_by DESC
    r#"
        CREATE TABLE test_sort_desc (
            id SERIAL PRIMARY KEY,
            description TEXT,
            rank INTEGER
        );

        CREATE INDEX test_sort_desc_idx ON test_sort_desc
        USING bm25 (id, description, rank)
        WITH (
            key_field = 'id',
            text_fields = '{"description": {}}',
            numeric_fields = '{"rank": {"fast": true}}',
            sort_by = 'rank DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert 6 batches to create 6 segments
    for batch in 1..=6 {
        let sql = format!(
            r#"
            INSERT INTO test_sort_desc (description, rank)
            SELECT
                'Document ' || i || ' batch{}',
                (random() * 50)::integer
            FROM generate_series(1, 20) AS i;
            "#,
            batch
        );
        sql.execute(&mut conn);
    }

    // Verify we have multiple segments
    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT id, rank FROM test_sort_desc
        WHERE description @@@ 'Document'
    "#
    .fetch_one(&mut conn);

    let segment_count = plan
        .pointer("/0/Plan/Plans/0/Segment Count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    assert!(
        segment_count > 1,
        "Test requires multiple segments, got {}",
        segment_count
    );

    // Query WITHOUT ORDER BY - results should still be sorted by rank DESC
    let results: Vec<(i32, i32)> = r#"
        SELECT id, rank FROM test_sort_desc
        WHERE description @@@ 'Document'
    "#
    .fetch(&mut conn);

    assert_eq!(
        results.len(),
        120,
        "Should return all 120 results (6 batches x 20)"
    );

    // Extract ranks and verify they are sorted DESC
    let ranks: Vec<i32> = results.iter().map(|(_, rank)| *rank).collect();
    assert!(
        is_sorted_desc(&ranks),
        "Results should be sorted by rank DESC when index has sort_by. First 20 ranks: {:?}",
        &ranks[..20.min(ranks.len())]
    );
}

/// Test sort_by with ASC order
#[rstest]
fn index_sort_by_asc_multi_segment(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    r#"
        CREATE TABLE test_sort_asc (
            id SERIAL PRIMARY KEY,
            description TEXT,
            score INTEGER
        );

        CREATE INDEX test_sort_asc_idx ON test_sort_asc
        USING bm25 (id, description, score)
        WITH (
            key_field = 'id',
            text_fields = '{"description": {}}',
            numeric_fields = '{"score": {"fast": true}}',
            sort_by = 'score ASC NULLS FIRST'
        );
    "#
    .execute(&mut conn);

    // Insert multiple batches to create segments
    for batch in 1..=4 {
        let sql = format!(
            r#"
            INSERT INTO test_sort_asc (description, score)
            SELECT
                'Item ' || i || ' batch{}',
                (random() * 100)::integer
            FROM generate_series(1, 25) AS i;
            "#,
            batch
        );
        sql.execute(&mut conn);
    }

    // Query WITHOUT ORDER BY
    let results: Vec<(i32, i32)> = r#"
        SELECT id, score FROM test_sort_asc
        WHERE description @@@ 'Item'
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 100, "Should return all 100 results");

    let scores: Vec<i32> = results.iter().map(|(_, score)| *score).collect();
    assert!(
        is_sorted_asc(&scores),
        "Results should be sorted by score ASC when index has sort_by. First 20 scores: {:?}",
        &scores[..20.min(scores.len())]
    );
}

/// Test that sort_by works correctly with LIMIT
#[rstest]
fn index_sort_by_with_limit(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    r#"
        CREATE TABLE test_sort_limit (
            id SERIAL PRIMARY KEY,
            content TEXT,
            priority INTEGER
        );

        CREATE INDEX test_sort_limit_idx ON test_sort_limit
        USING bm25 (id, content, priority)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}}',
            numeric_fields = '{"priority": {"fast": true}}',
            sort_by = 'priority DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert data with known priority values
    r#"
        INSERT INTO test_sort_limit (content, priority)
        SELECT 'Record ' || i, i
        FROM generate_series(1, 100) AS i;
    "#
    .execute(&mut conn);

    // Create more segments
    r#"
        INSERT INTO test_sort_limit (content, priority)
        SELECT 'Record extra ' || i, 100 + i
        FROM generate_series(1, 50) AS i;
    "#
    .execute(&mut conn);

    // Query with LIMIT - should get top 10 by priority DESC
    let results: Vec<(i32, i32)> = r#"
        SELECT id, priority FROM test_sort_limit
        WHERE content @@@ 'Record'
        LIMIT 10
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 10, "Should return exactly 10 results");

    let priorities: Vec<i32> = results.iter().map(|(_, p)| *p).collect();
    assert!(
        is_sorted_desc(&priorities),
        "Results should be sorted by priority DESC. Got: {:?}",
        priorities
    );

    // The top 10 should be the highest priorities (150, 149, 148, ...)
    assert!(
        priorities[0] >= 141,
        "First result should have high priority, got {}",
        priorities[0]
    );
}

/// Test that index without sort_by does NOT guarantee sorted output
#[rstest]
fn index_without_sort_by_not_sorted(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    r#"
        CREATE TABLE test_no_sort (
            id SERIAL PRIMARY KEY,
            description TEXT,
            value INTEGER
        );

        -- Index WITHOUT sort_by
        CREATE INDEX test_no_sort_idx ON test_no_sort
        USING bm25 (id, description, value)
        WITH (
            key_field = 'id',
            text_fields = '{"description": {}}',
            numeric_fields = '{"value": {"fast": true}}'
        );
    "#
    .execute(&mut conn);

    // Insert multiple batches
    for batch in 1..=5 {
        let sql = format!(
            r#"
            INSERT INTO test_no_sort (description, value)
            SELECT
                'Entry ' || i || ' batch{}',
                (random() * 100)::integer
            FROM generate_series(1, 20) AS i;
            "#,
            batch
        );
        sql.execute(&mut conn);
    }

    // Query WITHOUT ORDER BY
    let results: Vec<(i32, i32)> = r#"
        SELECT id, value FROM test_no_sort
        WHERE description @@@ 'Entry'
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 100, "Should return all 100 results");

    // Results should NOT necessarily be sorted (this is expected behavior)
    // We just verify the query works - sorting is not guaranteed
    let values: Vec<i32> = results.iter().map(|(_, v)| *v).collect();
    eprintln!("Without sort_by, first 20 values: {:?}", &values[..20]);
}

/// Test that explicit ORDER BY still works with sort_by index
#[rstest]
fn index_sort_by_with_explicit_order_by(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    r#"
        CREATE TABLE test_explicit_order (
            id SERIAL PRIMARY KEY,
            content TEXT,
            rank INTEGER,
            name TEXT
        );

        CREATE INDEX test_explicit_order_idx ON test_explicit_order
        USING bm25 (id, content, rank, name)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}, "name": {"fast": true, "tokenizer": {"type": "raw"}}}',
            numeric_fields = '{"rank": {"fast": true}}',
            sort_by = 'rank DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    r#"
        INSERT INTO test_explicit_order (content, rank, name) VALUES
        ('apple fruit', 10, 'Alpha'),
        ('banana fruit', 20, 'Beta'),
        ('cherry fruit', 5, 'Gamma'),
        ('date fruit', 15, 'Delta'),
        ('elderberry fruit', 25, 'Epsilon');
    "#
    .execute(&mut conn);

    // Create another segment
    r#"
        INSERT INTO test_explicit_order (content, rank, name) VALUES
        ('fig fruit', 8, 'Zeta'),
        ('grape fruit', 30, 'Eta'),
        ('honeydew fruit', 12, 'Theta');
    "#
    .execute(&mut conn);

    // Query with explicit ORDER BY that matches sort_by
    let results_match: Vec<(i32, String)> = r#"
        SELECT rank, name FROM test_explicit_order
        WHERE content @@@ 'fruit'
        ORDER BY rank DESC
    "#
    .fetch(&mut conn);

    let ranks_match: Vec<i32> = results_match.iter().map(|(r, _)| *r).collect();
    assert!(is_sorted_desc(&ranks_match), "Should be sorted DESC");

    // Query with explicit ORDER BY that differs from sort_by
    let results_diff: Vec<(i32, String)> = r#"
        SELECT rank, name FROM test_explicit_order
        WHERE content @@@ 'fruit'
        ORDER BY rank ASC
    "#
    .fetch(&mut conn);

    let ranks_diff: Vec<i32> = results_diff.iter().map(|(r, _)| *r).collect();
    assert!(
        is_sorted_asc(&ranks_diff),
        "Should be sorted ASC when explicit ORDER BY differs"
    );
}

/// Test concurrent inserts don't break sorted output
#[rstest]
fn index_sort_by_after_updates(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    r#"
        CREATE TABLE test_sort_updates (
            id SERIAL PRIMARY KEY,
            content TEXT,
            score INTEGER
        );

        CREATE INDEX test_sort_updates_idx ON test_sort_updates
        USING bm25 (id, content, score)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}}',
            numeric_fields = '{"score": {"fast": true}}',
            sort_by = 'score DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Initial insert
    r#"
        INSERT INTO test_sort_updates (content, score)
        SELECT 'Item ' || i, i * 10
        FROM generate_series(1, 20) AS i;
    "#
    .execute(&mut conn);

    // Verify initial sorted order
    let results1: Vec<(i32, i32)> = r#"
        SELECT id, score FROM test_sort_updates
        WHERE content @@@ 'Item'
    "#
    .fetch(&mut conn);

    let scores1: Vec<i32> = results1.iter().map(|(_, s)| *s).collect();
    assert!(is_sorted_desc(&scores1), "Initial results should be sorted");

    // Insert more data (creates new segment)
    r#"
        INSERT INTO test_sort_updates (content, score)
        SELECT 'Item extra ' || i, 50 + i
        FROM generate_series(1, 15) AS i;
    "#
    .execute(&mut conn);

    // Verify still sorted after insert
    let results2: Vec<(i32, i32)> = r#"
        SELECT id, score FROM test_sort_updates
        WHERE content @@@ 'Item'
    "#
    .fetch(&mut conn);

    let scores2: Vec<i32> = results2.iter().map(|(_, s)| *s).collect();
    assert!(
        is_sorted_desc(&scores2),
        "Results should remain sorted after insert. First 20: {:?}",
        &scores2[..20.min(scores2.len())]
    );

    // Update some scores
    "UPDATE test_sort_updates SET score = 999 WHERE id = 1".execute(&mut conn);
    "UPDATE test_sort_updates SET score = 1 WHERE id = 20".execute(&mut conn);

    // Verify sorted after update
    let results3: Vec<(i32, i32)> = r#"
        SELECT id, score FROM test_sort_updates
        WHERE content @@@ 'Item'
    "#
    .fetch(&mut conn);

    let scores3: Vec<i32> = results3.iter().map(|(_, s)| *s).collect();
    assert!(
        is_sorted_desc(&scores3),
        "Results should remain sorted after update. First 10: {:?}",
        &scores3[..10.min(scores3.len())]
    );

    // The updated row with score=999 should be first
    assert_eq!(
        scores3[0], 999,
        "Updated row with highest score should be first"
    );
}
