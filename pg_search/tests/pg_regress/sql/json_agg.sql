-- =====================================================================
-- Test JSON projection aggregation
-- =====================================================================
-- This test verifies that aggregate scan works with JSON projection operators

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Create test table with JSON data
CREATE TABLE json_test (
    id SERIAL PRIMARY KEY,
    metadata_json JSONB
);

-- Insert sample data
INSERT INTO json_test (metadata_json) VALUES
    ('{"value": "apple", "count": 5}'::JSONB),
    ('{"value": "banana", "count": 3}'::JSONB),
    ('{"value": "apple", "count": 2}'::JSONB),
    ('{"value": "orange", "count": 7}'::JSONB),
    ('{"value": "banana", "count": 1}'::JSONB),
    ('{"value": "apple", "count": 4}'::JSONB),
    ('{"value": "cherry", "count": 6}'::JSONB),
    ('{"value": "banana", "count": 8}'::JSONB);

-- Create BM25 index with JSON field as fast field
CREATE INDEX json_test_idx ON json_test
USING bm25 (id, metadata_json)
WITH (
    key_field = 'id',
    json_fields = '{"metadata_json": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test 1: Basic JSON projection aggregation with ->> operator
-- This should use the aggregate scan for fast aggregation
SELECT 'Test 1: JSON projection with ->>';
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT
    metadata_json->>'value' AS value,
    COUNT(*) AS count
FROM json_test
WHERE id @@@ paradedb.exists('metadata_json.value')
GROUP BY metadata_json->>'value'
ORDER BY count DESC, value;

-- Verify the results
SELECT
    metadata_json->>'value' AS value,
    COUNT(*) AS count
FROM json_test
WHERE id @@@ paradedb.exists('metadata_json.value')
GROUP BY metadata_json->>'value'
ORDER BY count DESC, value;

-- Test 2: JSON projection with -> operator (returns JSON)
SELECT 'Test 2: JSON projection with ->';
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT
    metadata_json->'value' AS value,
    COUNT(*) AS count
FROM json_test
WHERE id @@@ paradedb.exists('metadata_json.value')
GROUP BY metadata_json->'value'
ORDER BY count DESC;

SELECT
    metadata_json->'value' AS value,
    COUNT(*) AS count
FROM json_test
WHERE id @@@ paradedb.exists('metadata_json.value')
GROUP BY metadata_json->'value'
ORDER BY count DESC;

-- Test 3: Multiple aggregates with JSON projection
SELECT 'Test 3: Multiple aggregates';
SELECT
    metadata_json->>'value' AS value,
    COUNT(*) AS count,
    MIN((metadata_json->>'count')::INT) AS min_count,
    MAX((metadata_json->>'count')::INT) AS max_count
FROM json_test
WHERE id @@@ paradedb.exists('metadata_json.value')
GROUP BY metadata_json->>'value'
ORDER BY value;

-- Test 4: Verify with paradedb.aggregate directly
SELECT 'Test 4: Direct paradedb.aggregate call';
SELECT * FROM paradedb.aggregate(
    index=>'json_test_idx',
    query=>paradedb.exists('metadata_json.value'),
    agg=>'{"buckets": { "terms": { "field": "metadata_json.value" }}}',
    solve_mvcc=>true
) ORDER BY 1;

-- Test 5: JSON projection without filter (should still work)
SELECT 'Test 5: Without WHERE clause';
SELECT
    metadata_json->>'value' AS value,
    COUNT(*) AS count
FROM json_test
GROUP BY metadata_json->>'value'
ORDER BY value;

-- Cleanup
DROP TABLE json_test;
