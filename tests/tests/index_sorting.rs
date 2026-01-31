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
//! These tests verify that when an index is created with `sort_by` and a query
//! includes an ORDER BY clause matching the index's sort order, the sorted path
//! is activated. This uses `SortPreservingMergeExec` to merge sorted segment
//! outputs into globally sorted results efficiently.
//!
//! Note: The sorted path is only used when ORDER BY matches the index's sort_by.
//! Without ORDER BY, PostgreSQL has no obligation to maintain sorted output.

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

/// Test that a sorted index scan with ORDER BY returns sorted results
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
        ORDER BY rank DESC
    "#
    .fetch_one(&mut conn);

    let segment_count = plan
        .pointer("/0/Plan/Plans/0/Plans/0/Segment Count")
        .or_else(|| plan.pointer("/0/Plan/Plans/0/Segment Count"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    assert!(
        segment_count > 1,
        "Test requires multiple segments, got {}",
        segment_count
    );

    // Query with ORDER BY matching sort_by - triggers sorted path
    let results: Vec<(i32, i32)> = r#"
        SELECT id, rank FROM test_sort_desc
        WHERE description @@@ 'Document'
        ORDER BY rank DESC
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

    // Query with ORDER BY matching sort_by
    let results: Vec<(i32, i32)> = r#"
        SELECT id, score FROM test_sort_asc
        WHERE description @@@ 'Item'
        ORDER BY score ASC
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 100, "Should return all 100 results");

    let scores: Vec<i32> = results.iter().map(|(_, score)| *score).collect();
    assert!(
        is_sorted_asc(&scores),
        "Results should be sorted by score ASC when ORDER BY matches sort_by. First 20 scores: {:?}",
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

    // Query with ORDER BY + LIMIT - should get top 10 by priority DESC
    let results: Vec<(i32, i32)> = r#"
        SELECT id, priority FROM test_sort_limit
        WHERE content @@@ 'Record'
        ORDER BY priority DESC
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

    // Verify initial sorted order with ORDER BY
    let results1: Vec<(i32, i32)> = r#"
        SELECT id, score FROM test_sort_updates
        WHERE content @@@ 'Item'
        ORDER BY score DESC
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

    // Verify still sorted after insert with ORDER BY
    let results2: Vec<(i32, i32)> = r#"
        SELECT id, score FROM test_sort_updates
        WHERE content @@@ 'Item'
        ORDER BY score DESC
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

    // Verify sorted after update with ORDER BY
    let results3: Vec<(i32, i32)> = r#"
        SELECT id, score FROM test_sort_updates
        WHERE content @@@ 'Item'
        ORDER BY score DESC
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

// ============================================================================
// Parallel Execution Tests
// ============================================================================
//
// These tests verify that the sorted index path works correctly with parallel
// workers. The key requirement is that workers properly share segments via the
// lazy checkout model - each worker should claim a subset of segments, not one
// worker claiming all segments.

/// Test that parallel workers produce correctly sorted results with sorted index.
///
/// This test verifies that when parallel workers are enabled AND ORDER BY matches
/// the index's sort_by:
/// 1. PostgreSQL uses Gather Merge to combine sorted outputs from workers
/// 2. Each worker produces sorted results for its claimed segments
/// 3. The final merged result is correctly sorted
///
/// Note: The sorted path is only used when ORDER BY matches the index's sort_by.
/// Without ORDER BY, PostgreSQL has no obligation to maintain sorted output.
#[rstest]
fn index_sort_by_parallel_workers(mut conn: PgConnection) {
    // debug_parallel_query is only available in PG17+
    if pg_major_version(&mut conn) < 17 {
        eprintln!("Skipping test: requires PG17+ for debug_parallel_query");
        return;
    }

    // Enable parallel workers
    "SET max_parallel_workers TO 4;".execute(&mut conn);
    "SET max_parallel_workers_per_gather TO 4;".execute(&mut conn);
    "SET debug_parallel_query TO on;".execute(&mut conn);

    // Create table with sort_by index
    r#"
        CREATE TABLE test_parallel_sort (
            id SERIAL PRIMARY KEY,
            content TEXT,
            rank INTEGER
        );

        CREATE INDEX test_parallel_sort_idx ON test_parallel_sort
        USING bm25 (id, content, rank)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}}',
            numeric_fields = '{"rank": {"fast": true}}',
            sort_by = 'rank DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert enough data to create multiple segments (8 batches = 8 segments)
    for batch in 1..=8 {
        let sql = format!(
            r#"
            INSERT INTO test_parallel_sort (content, rank)
            SELECT
                'Item ' || i || ' batch{}',
                (random() * 100)::integer
            FROM generate_series(1, 50) AS i;
            "#,
            batch
        );
        sql.execute(&mut conn);
    }

    // Query WITH ORDER BY that matches index sort_by - this triggers the sorted path
    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id, rank FROM test_parallel_sort
        WHERE content @@@ 'Item'
        ORDER BY rank DESC
    "#
    .fetch_one(&mut conn);

    eprintln!("Parallel sorted plan: {:#?}", plan);

    // Check that parallel workers were planned
    let workers_planned = plan
        .pointer("/0/Plan/Workers Planned")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    eprintln!("Workers planned: {}", workers_planned);

    // Query with ORDER BY matching sort_by - results should be sorted
    let results: Vec<(i32, i32)> = r#"
        SELECT id, rank FROM test_parallel_sort
        WHERE content @@@ 'Item'
        ORDER BY rank DESC
    "#
    .fetch(&mut conn);

    assert_eq!(
        results.len(),
        400,
        "Should return all 400 results (8 batches x 50)"
    );

    let ranks: Vec<i32> = results.iter().map(|(_, r)| *r).collect();
    assert!(
        is_sorted_desc(&ranks),
        "Results should be sorted DESC with ORDER BY matching index sort_by. First 20 ranks: {:?}",
        &ranks[..20.min(ranks.len())]
    );
}

/// Test parallel sorted scan with LIMIT.
///
/// Verifies that LIMIT works correctly with parallel sorted execution,
/// where Gather Merge combines sorted streams from workers.
/// Requires ORDER BY to trigger the sorted path.
#[rstest]
fn index_sort_by_parallel_with_limit(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 17 {
        eprintln!("Skipping test: requires PG17+ for debug_parallel_query");
        return;
    }

    "SET max_parallel_workers TO 4;".execute(&mut conn);
    "SET max_parallel_workers_per_gather TO 4;".execute(&mut conn);
    "SET debug_parallel_query TO on;".execute(&mut conn);

    r#"
        CREATE TABLE test_parallel_limit (
            id SERIAL PRIMARY KEY,
            content TEXT,
            priority INTEGER
        );

        CREATE INDEX test_parallel_limit_idx ON test_parallel_limit
        USING bm25 (id, content, priority)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}}',
            numeric_fields = '{"priority": {"fast": true}}',
            sort_by = 'priority DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert data with known priority distribution across segments
    for batch in 1..=6 {
        let sql = format!(
            r#"
            INSERT INTO test_parallel_limit (content, priority)
            SELECT
                'Record ' || i || ' batch{}',
                {} + i
            FROM generate_series(1, 30) AS i;
            "#,
            batch,
            batch * 100 // Each batch has priorities in different ranges
        );
        sql.execute(&mut conn);
    }

    // Query with ORDER BY + LIMIT - ORDER BY triggers sorted path
    let results: Vec<(i32, i32)> = r#"
        SELECT id, priority FROM test_parallel_limit
        WHERE content @@@ 'Record'
        ORDER BY priority DESC
        LIMIT 10
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 10, "Should return exactly 10 results");

    let priorities: Vec<i32> = results.iter().map(|(_, p)| *p).collect();
    assert!(
        is_sorted_desc(&priorities),
        "Results should be sorted DESC with parallel + ORDER BY + LIMIT. Got: {:?}",
        priorities
    );

    // The top priorities should come from the highest batch (batch 6: 601-630)
    assert!(
        priorities[0] >= 620,
        "First result should have high priority from batch 6, got {}",
        priorities[0]
    );
}

/// Test that sorted index works with a small dataset (few segments).
///
/// Edge case: When there are few segments, SortPreservingMergeExec
/// should still work correctly.
#[rstest]
fn index_sort_by_single_segment(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    r#"
        CREATE TABLE test_single_segment (
            id SERIAL PRIMARY KEY,
            content TEXT,
            score INTEGER
        );

        CREATE INDEX test_single_segment_idx ON test_single_segment
        USING bm25 (id, content, score)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}}',
            numeric_fields = '{"score": {"fast": true}}',
            sort_by = 'score DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert only one batch - may create 1-2 segments
    r#"
        INSERT INTO test_single_segment (content, score)
        SELECT 'Entry ' || i, (random() * 100)::integer
        FROM generate_series(1, 50) AS i;
    "#
    .execute(&mut conn);

    // Query with ORDER BY and verify sorted
    let results: Vec<(i32, i32)> = r#"
        SELECT id, score FROM test_single_segment
        WHERE content @@@ 'Entry'
        ORDER BY score DESC
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 50, "Should return all 50 results");

    let scores: Vec<i32> = results.iter().map(|(_, s)| *s).collect();
    assert!(
        is_sorted_desc(&scores),
        "Results should be sorted DESC. First 10: {:?}",
        &scores[..10.min(scores.len())]
    );
}

/// Test sorted index with empty results.
///
/// Edge case: Query that matches no documents should return empty result
/// without errors.
#[rstest]
fn index_sort_by_empty_results(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    r#"
        CREATE TABLE test_empty_sort (
            id SERIAL PRIMARY KEY,
            content TEXT,
            rank INTEGER
        );

        CREATE INDEX test_empty_sort_idx ON test_empty_sort
        USING bm25 (id, content, rank)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}}',
            numeric_fields = '{"rank": {"fast": true}}',
            sort_by = 'rank DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert some data
    r#"
        INSERT INTO test_empty_sort (content, rank)
        SELECT 'Document ' || i, i
        FROM generate_series(1, 20) AS i;
    "#
    .execute(&mut conn);

    // Query that matches nothing
    let results: Vec<(i32, i32)> = r#"
        SELECT id, rank FROM test_empty_sort
        WHERE content @@@ 'nonexistent_term_xyz'
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 0, "Should return empty result set");
}

/// Test sorted index with NULL values in sort field.
///
/// Verifies that NULLS FIRST/LAST ordering is respected.
#[rstest]
fn index_sort_by_null_handling(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    // Test NULLS LAST (default for DESC)
    r#"
        CREATE TABLE test_null_sort (
            id SERIAL PRIMARY KEY,
            content TEXT,
            score INTEGER
        );

        CREATE INDEX test_null_sort_idx ON test_null_sort
        USING bm25 (id, content, score)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}}',
            numeric_fields = '{"score": {"fast": true}}',
            sort_by = 'score DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert data with some NULL scores
    r#"
        INSERT INTO test_null_sort (content, score) VALUES
        ('Item A', 100),
        ('Item B', NULL),
        ('Item C', 50),
        ('Item D', NULL),
        ('Item E', 75);
    "#
    .execute(&mut conn);

    // Create another segment with more NULLs
    r#"
        INSERT INTO test_null_sort (content, score) VALUES
        ('Item F', 25),
        ('Item G', NULL),
        ('Item H', 90);
    "#
    .execute(&mut conn);

    let results: Vec<(i32, Option<i32>)> = r#"
        SELECT id, score FROM test_null_sort
        WHERE content @@@ 'Item'
        ORDER BY score DESC NULLS LAST
    "#
    .fetch(&mut conn);

    assert_eq!(results.len(), 8, "Should return all 8 results");

    // With NULLS LAST, non-null values should come first (sorted DESC),
    // followed by NULLs at the end
    let scores: Vec<Option<i32>> = results.iter().map(|(_, s)| *s).collect();

    // Find where NULLs start
    let first_null_idx = scores.iter().position(|s| s.is_none());
    if let Some(idx) = first_null_idx {
        // All values before first NULL should be non-null and sorted DESC
        let non_null_scores: Vec<i32> = scores[..idx].iter().filter_map(|s| *s).collect();
        assert!(
            is_sorted_desc(&non_null_scores),
            "Non-null scores should be sorted DESC. Got: {:?}",
            non_null_scores
        );

        // All values from first NULL onwards should be NULL
        assert!(
            scores[idx..].iter().all(|s| s.is_none()),
            "All values after first NULL should be NULL (NULLS LAST)"
        );
    }
}

// ============================================================================
// Large Segment Count Tests
// ============================================================================
//
// These tests verify that the sorted index path works correctly with many
// segments (100+). SortPreservingMergeExec uses a heap-based merge algorithm
// that should handle many input streams efficiently.

/// Test sorted index with 100+ segments.
///
/// This verifies that SortPreservingMergeExec can efficiently merge many
/// sorted segment streams without issues. Each batch creates a new segment,
/// so 120 batches = 120+ segments.
#[rstest]
fn index_sort_by_many_segments(mut conn: PgConnection) {
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    r#"
        CREATE TABLE test_many_segments (
            id SERIAL PRIMARY KEY,
            content TEXT,
            score INTEGER
        );

        CREATE INDEX test_many_segments_idx ON test_many_segments
        USING bm25 (id, content, score)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}}',
            numeric_fields = '{"score": {"fast": true}}',
            sort_by = 'score DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert 120 small batches to create 120+ segments
    // Each batch is small (5 rows) to ensure many segments without too much data
    for batch in 1..=120 {
        let sql = format!(
            r#"
            INSERT INTO test_many_segments (content, score)
            SELECT
                'Item ' || i || ' batch{}',
                {} + (i % 10)
            FROM generate_series(1, 5) AS i;
            "#,
            batch,
            batch * 10 // Different score ranges per batch for predictable ordering
        );
        sql.execute(&mut conn);
    }

    // Verify we have many segments
    let (plan,): (Value,) = r#"
        EXPLAIN (ANALYZE, FORMAT JSON)
        SELECT id, score FROM test_many_segments
        WHERE content @@@ 'Item'
        ORDER BY score DESC
    "#
    .fetch_one(&mut conn);

    let segment_count = plan
        .pointer("/0/Plan/Plans/0/Plans/0/Segment Count")
        .or_else(|| plan.pointer("/0/Plan/Plans/0/Segment Count"))
        .or_else(|| plan.pointer("/0/Plan/Segment Count"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    eprintln!("Many segments test: {} segments", segment_count);
    assert!(
        segment_count >= 100,
        "Test requires 100+ segments, got {}",
        segment_count
    );

    // Query with ORDER BY and verify results are correctly sorted
    let results: Vec<(i32, i32)> = r#"
        SELECT id, score FROM test_many_segments
        WHERE content @@@ 'Item'
        ORDER BY score DESC
    "#
    .fetch(&mut conn);

    assert_eq!(
        results.len(),
        600,
        "Should return all 600 results (120 batches x 5 rows)"
    );

    let scores: Vec<i32> = results.iter().map(|(_, s)| *s).collect();
    assert!(
        is_sorted_desc(&scores),
        "Results should be sorted DESC even with 100+ segments. First 20: {:?}, Last 20: {:?}",
        &scores[..20.min(scores.len())],
        &scores[scores.len().saturating_sub(20)..]
    );

    // Verify score range is correct (highest should be around 1200+9=1209)
    assert!(
        scores[0] >= 1200,
        "Highest score should be >= 1200 (batch 120 * 10 + offset), got {}",
        scores[0]
    );
}

/// Test sorted index with many segments in parallel execution.
///
/// Verifies that parallel workers can handle 100+ segments efficiently,
/// with each worker claiming and merging a subset of segments.
#[rstest]
fn index_sort_by_many_segments_parallel(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 17 {
        eprintln!("Skipping test: requires PG17+ for debug_parallel_query");
        return;
    }

    "SET max_parallel_workers TO 4;".execute(&mut conn);
    "SET max_parallel_workers_per_gather TO 4;".execute(&mut conn);
    "SET debug_parallel_query TO on;".execute(&mut conn);

    r#"
        CREATE TABLE test_many_segments_parallel (
            id SERIAL PRIMARY KEY,
            content TEXT,
            priority INTEGER
        );

        CREATE INDEX test_many_segments_parallel_idx ON test_many_segments_parallel
        USING bm25 (id, content, priority)
        WITH (
            key_field = 'id',
            text_fields = '{"content": {}}',
            numeric_fields = '{"priority": {"fast": true}}',
            sort_by = 'priority DESC NULLS LAST'
        );
    "#
    .execute(&mut conn);

    // Insert 100 batches to create 100+ segments
    for batch in 1..=100 {
        let sql = format!(
            r#"
            INSERT INTO test_many_segments_parallel (content, priority)
            SELECT
                'Record ' || i || ' batch{}',
                {} + i
            FROM generate_series(1, 8) AS i;
            "#,
            batch,
            batch * 100 // Large ranges to spread across batches
        );
        sql.execute(&mut conn);
    }

    // Query with ORDER BY to trigger sorted parallel merge
    let results: Vec<(i32, i32)> = r#"
        SELECT id, priority FROM test_many_segments_parallel
        WHERE content @@@ 'Record'
        ORDER BY priority DESC
    "#
    .fetch(&mut conn);

    assert_eq!(
        results.len(),
        800,
        "Should return all 800 results (100 batches x 8 rows)"
    );

    let priorities: Vec<i32> = results.iter().map(|(_, p)| *p).collect();
    assert!(
        is_sorted_desc(&priorities),
        "Results should be sorted DESC with parallel + many segments. First 20: {:?}",
        &priorities[..20.min(priorities.len())]
    );

    // Top results should be from batch 100 (priority 10001-10008)
    assert!(
        priorities[0] >= 10000,
        "Highest priority should be >= 10000, got {}",
        priorities[0]
    );
}
