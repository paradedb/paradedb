-- Test pdb.agg() wrapped in functions and CTEs
-- This test verifies that pdb.agg() works correctly when:
-- 1. Wrapped in another function call (like jsonb_pretty)
-- 2. Used in a CTE

-- Cleanup if exists
DROP TABLE IF EXISTS fn_wrapped_agg_logs CASCADE;

-- First, create the test table
CREATE TABLE fn_wrapped_agg_logs (
    log_id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    timestamp TIMESTAMP DEFAULT NOW()
);

-- Create a BM25 index with fast fields for aggregation
CREATE INDEX fn_wrapped_agg_logs_idx ON fn_wrapped_agg_logs
USING bm25 (log_id, description, category)
WITH (
    key_field = 'log_id',
    text_fields = '{
        "description": {},
        "category": {"fast": true}
    }'
);

-- Insert test data
INSERT INTO fn_wrapped_agg_logs (description, category) VALUES
    ('error in application', 'app'),
    ('error in database', 'db'),
    ('warning message', 'app'),
    ('error in network', 'network'),
    ('info message', 'app');

-- Test 1: Basic pdb.agg() with window function (should work)
-- Use paradedb.all() to force custom scan
-- Only select indexed fields (log_id is the key_field)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT log_id, description, category, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () 
FROM fn_wrapped_agg_logs
WHERE description @@@ paradedb.all()
ORDER BY log_id DESC LIMIT 3;

SELECT log_id, description, category, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () 
FROM fn_wrapped_agg_logs
WHERE description @@@ paradedb.all()
ORDER BY log_id DESC LIMIT 3;

-- Test 2: pdb.agg() wrapped in jsonb_pretty
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT log_id, description, category, jsonb_pretty(pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()) 
FROM fn_wrapped_agg_logs
WHERE description @@@ paradedb.all()
ORDER BY log_id DESC LIMIT 3;

SELECT log_id, description, category, jsonb_pretty(pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()) 
FROM fn_wrapped_agg_logs
WHERE description @@@ paradedb.all()
ORDER BY log_id DESC LIMIT 3;

-- Test 3: pdb.agg() in CTE
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
WITH agg AS (
    SELECT log_id, description, category, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () as agg_result
    FROM fn_wrapped_agg_logs
    WHERE description @@@ paradedb.all()
    ORDER BY log_id DESC LIMIT 3
)
SELECT * FROM agg;

WITH agg AS (
    SELECT log_id, description, category, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () as agg_result
    FROM fn_wrapped_agg_logs
    WHERE description @@@ paradedb.all()
    ORDER BY log_id DESC LIMIT 3
)
SELECT * FROM agg;

-- Test 4: pdb.agg() wrapped in CTE and then wrapped in function
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
WITH agg AS (
    SELECT log_id, description, category, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () as agg_result
    FROM fn_wrapped_agg_logs
    WHERE description @@@ paradedb.all()
    ORDER BY log_id DESC LIMIT 3
)
SELECT log_id, description, category, jsonb_pretty(agg_result) FROM agg;

WITH agg AS (
    SELECT log_id, description, category, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () as agg_result
    FROM fn_wrapped_agg_logs
    WHERE description @@@ paradedb.all()
    ORDER BY log_id DESC LIMIT 3
)
SELECT log_id, description, category, jsonb_pretty(agg_result) FROM agg;

-- =====================================================================
-- Non-Window Aggregates Wrapped in Functions (Issue #3757)
-- =====================================================================

SET paradedb.enable_aggregate_custom_scan TO on;

-- Add a numeric column for aggregate tests
ALTER TABLE fn_wrapped_agg_logs ADD COLUMN score INTEGER DEFAULT 50;
UPDATE fn_wrapped_agg_logs SET score = log_id * 10;

-- Recreate index with score column using v2 API
DROP INDEX fn_wrapped_agg_logs_idx;
CREATE INDEX fn_wrapped_agg_logs_idx ON fn_wrapped_agg_logs
USING bm25 (log_id, description, (category::pdb.literal), score)
WITH (key_field = 'log_id');

-- Test 5: pdb.agg() wrapped in jsonb_pretty (non-window)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT jsonb_pretty(pdb.agg('{"value_count": {"field": "score"}}'))
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

SELECT jsonb_pretty(pdb.agg('{"value_count": {"field": "score"}}'))
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

-- Test 6: pdb.agg() with -> operator (non-window)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT (pdb.agg('{"stats": {"field": "score"}}'))->'avg' as avg_score
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

SELECT (pdb.agg('{"stats": {"field": "score"}}'))->'avg' as avg_score
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

-- Test 7: pdb.agg() wrapped in COALESCE (non-window)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COALESCE(pdb.agg('{"value_count": {"field": "score"}}'), '{}')
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

SELECT COALESCE(pdb.agg('{"value_count": {"field": "score"}}'), '{}')
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

-- Test 8: Nested wrappers - jsonb_pretty(COALESCE(pdb.agg(...)))
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT jsonb_pretty(COALESCE(pdb.agg('{"value_count": {"field": "score"}}'), '{}'))
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

SELECT jsonb_pretty(COALESCE(pdb.agg('{"value_count": {"field": "score"}}'), '{}'))
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

-- Test 9: Standard COUNT(*) wrapped in COALESCE (non-window)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COALESCE(COUNT(*), 0)
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

SELECT COALESCE(COUNT(*), 0)
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

-- Test 10: Standard SUM wrapped in COALESCE (non-window)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COALESCE(SUM(score), 0)
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

SELECT COALESCE(SUM(score), 0)
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error';

RESET paradedb.enable_aggregate_custom_scan;

-- Cleanup
DROP TABLE fn_wrapped_agg_logs CASCADE;

