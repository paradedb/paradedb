-- Test field validation for pdb.agg()
-- This tests that invalid/non-indexed fields are detected and reported

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Helper function to sort buckets by key for stable test output
-- (bucket order with same doc_count is not deterministic)
CREATE OR REPLACE FUNCTION sort_agg_buckets(agg jsonb) RETURNS jsonb AS $$
SELECT CASE 
    WHEN agg->'buckets' IS NOT NULL THEN
        jsonb_build_object(
            'buckets', (
                SELECT jsonb_agg(bucket ORDER BY (bucket->>'key')::numeric)
                FROM jsonb_array_elements(agg->'buckets') AS bucket
            ),
            'sum_other_doc_count', agg->'sum_other_doc_count',
            'doc_count_error_upper_bound', agg->'doc_count_error_upper_bound'
        )
    ELSE agg
END;
$$ LANGUAGE SQL IMMUTABLE;

DROP TABLE IF EXISTS mock_items CASCADE;

-- Setup test data
CREATE TABLE mock_items (
    id SERIAL PRIMARY KEY,
    description TEXT,
    rating INT,
    created_at TIMESTAMP
);

INSERT INTO mock_items (description, rating, created_at) VALUES
    ('Ergonomic keyboard', 5, '2024-01-01 10:00:00'),
    ('Wireless mouse', 4, '2024-01-02 11:00:00'),
    ('USB hub', 3, '2024-01-03 12:00:00'),
    ('Monitor stand', 5, '2024-01-04 13:00:00'),
    ('Laptop bag', 4, '2024-01-05 14:00:00');

-- Create index with specific fields
CREATE INDEX mock_items_idx ON mock_items
USING bm25 (id, description, rating, created_at)
WITH (
    key_field = 'id',
    text_fields = '{"description": {}}',
    numeric_fields = '{"rating": {"fast": true}}'
);

-- =====================================================================
-- SECTION 1: Valid field references (should succeed)
-- =====================================================================

-- Test 1: Valid field in avg aggregation
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"avg": {"field": "rating"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

SELECT pdb.agg('{"avg": {"field": "rating"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 2: Valid field in terms aggregation (wrapped for stable bucket order)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT sort_agg_buckets(agg) FROM (
    SELECT pdb.agg('{"terms": {"field": "rating"}}'::jsonb) as agg
    FROM mock_items
    WHERE id @@@ pdb.all()
) sub;

SELECT sort_agg_buckets(agg) FROM (
    SELECT pdb.agg('{"terms": {"field": "rating"}}'::jsonb) as agg
    FROM mock_items
    WHERE id @@@ pdb.all()
) sub;

-- Test 3: Valid field in date_histogram aggregation
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"date_histogram": {"field": "created_at", "fixed_interval": "30d"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

SELECT pdb.agg('{"date_histogram": {"field": "created_at", "fixed_interval": "30d"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 4: Valid field in window function context
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT *, pdb.agg('{"avg": {"field": "rating"}}'::jsonb) OVER ()
FROM mock_items
WHERE id @@@ pdb.all()
ORDER BY id DESC LIMIT 3;

SELECT *, pdb.agg('{"avg": {"field": "rating"}}'::jsonb) OVER ()
FROM mock_items
WHERE id @@@ pdb.all()
ORDER BY id DESC LIMIT 3;

-- =====================================================================
-- SECTION 2: Invalid field references (should error)
-- =====================================================================

-- Test 5: Invalid field in GROUP BY context - should error
SELECT pdb.agg('{"avg": {"field": "not_valid"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 6: Invalid field in date_histogram - should error
SELECT pdb.agg('{"date_histogram": {"field": "not_valid", "fixed_interval": "30d"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 7: Invalid field in terms aggregation - should error
SELECT pdb.agg('{"terms": {"field": "nonexistent_column"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 8: Invalid field in window function context - should error
SELECT *, pdb.agg('{"avg": {"field": "no_such_field"}}'::jsonb) OVER ()
FROM mock_items
WHERE id @@@ pdb.all()
ORDER BY id DESC LIMIT 3;

-- Test 9: Invalid field in range aggregation - should error
SELECT pdb.agg('{"range": {"field": "invalid_field", "ranges": [{"to": 3}, {"from": 3}]}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- =====================================================================
-- SECTION 3: Nested aggregations with invalid fields
-- =====================================================================

-- Test 10: Invalid field in nested aggregation - should error
SELECT pdb.agg('{"terms": {"field": "rating"}, "aggs": {"avg_invalid": {"avg": {"field": "bad_field"}}}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 11: Multiple levels of nesting with invalid field - should error
SELECT pdb.agg('{"terms": {"field": "rating"}, "aggs": {"nested": {"terms": {"field": "also_invalid"}}}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- =====================================================================
-- SECTION 4: Valid nested aggregations (should succeed)
-- =====================================================================

-- Test 12: Valid nested aggregation (wrapped for stable bucket order)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT sort_agg_buckets(agg) FROM (
    SELECT pdb.agg('{"terms": {"field": "rating"}, "aggs": {"avg_rating": {"avg": {"field": "rating"}}}}'::jsonb) as agg
    FROM mock_items
    WHERE id @@@ pdb.all()
) sub;

SELECT sort_agg_buckets(agg) FROM (
    SELECT pdb.agg('{"terms": {"field": "rating"}, "aggs": {"avg_rating": {"avg": {"field": "rating"}}}}'::jsonb) as agg
    FROM mock_items
    WHERE id @@@ pdb.all()
) sub;

-- =====================================================================
-- SECTION 5: paradedb.aggregate() direct API - field validation
-- This path bypasses the planner, so field validation happens in aggregate_impl()
-- =====================================================================

-- Test 13: Valid field via paradedb.aggregate() - should succeed
SELECT * FROM paradedb.aggregate(index=>'mock_items_idx', query=>paradedb.all(), agg=>'{"avg_rating": {"avg": {"field": "rating"}}}');

-- Test 14: Invalid field via paradedb.aggregate() - should error
SELECT * FROM paradedb.aggregate(index=>'mock_items_idx', query=>paradedb.all(), agg=>'{"avg_bad": {"avg": {"field": "nonexistent_field"}}}');

-- Test 15: Invalid nested field via paradedb.aggregate() - should error
SELECT * FROM paradedb.aggregate(index=>'mock_items_idx', query=>paradedb.all(), agg=>'{"by_rating": {"terms": {"field": "rating"}, "aggs": {"bad_avg": {"avg": {"field": "no_such_field"}}}}}');

-- =====================================================================
-- SECTION 6: Single-value metrics on datetime fields
-- These should attach a `key_as_string` ISO timestamp next to the raw
-- i64 micros `value`, mirroring the bucket key_as_string convention.
-- =====================================================================

-- Test 16: min on a datetime field
SELECT pdb.agg('{"min": {"field": "created_at"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 17: max on a datetime field
SELECT pdb.agg('{"max": {"field": "created_at"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 18: avg on a datetime field
SELECT pdb.agg('{"avg": {"field": "created_at"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 19: sum on a datetime field (also gets key_as_string even though it's
-- not a meaningful timestamp — the rewriter applies uniformly across min/max/sum/avg)
SELECT pdb.agg('{"sum": {"field": "created_at"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 20: single-value metric on a non-datetime field — no key_as_string
SELECT pdb.agg('{"min": {"field": "rating"}}'::jsonb)
FROM mock_items
WHERE id @@@ pdb.all();

-- Test 21: nested sub-agg: terms over rating with a min on the datetime field —
-- the sub-agg should also pick up key_as_string
SELECT sort_agg_buckets(agg) FROM (
    SELECT pdb.agg('{"terms": {"field": "rating"}, "aggs": {"earliest": {"min": {"field": "created_at"}}}}'::jsonb) as agg
    FROM mock_items
    WHERE id @@@ pdb.all()
) sub;

-- Test 22: same key_as_string behavior via paradedb.aggregate() direct API
SELECT * FROM paradedb.aggregate(index=>'mock_items_idx', query=>paradedb.all(), agg=>'{"first_seen": {"min": {"field": "created_at"}}}');

-- Cleanup
DROP TABLE mock_items CASCADE;
DROP FUNCTION sort_agg_buckets(jsonb);

