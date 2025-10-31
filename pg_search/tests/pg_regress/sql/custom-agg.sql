-- Test custom agg function with pdb.agg()

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS logs CASCADE;

-- Setup test data
CREATE TABLE logs (
    id SERIAL PRIMARY KEY,
    description TEXT,
    severity TEXT,
    category TEXT,
    response_time INT,
    status_code INT,
    timestamp TIMESTAMP
);

INSERT INTO logs (description, severity, category, response_time, status_code, timestamp) VALUES
    ('Database connection error', 'error', 'database', 150, 500, '2024-01-01 10:00:00'),
    ('Failed to fetch data', 'error', 'api', 200, 404, '2024-01-01 10:01:00'),
    ('Timeout connecting to service', 'error', 'network', 5000, 503, '2024-01-01 10:02:00'),
    ('Invalid query syntax error', 'error', 'database', 50, 400, '2024-01-01 10:03:00');

CREATE INDEX logs_idx ON logs USING bm25 (id, description, severity, category, response_time, status_code, timestamp)
WITH (
    key_field = 'id',
    text_fields = '{"description": {}, "severity": {"fast": true}, "category": {"fast": true}}',
    numeric_fields = '{"response_time": {"fast": true}, "status_code": {"fast": true}}',
    datetime_fields = '{"timestamp": {"fast": true}}'
);

-- Test 1: Simple custom agg with terms aggregation (without search query - should fail gracefully or not be intercepted)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, pdb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category, pdb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 2: Custom agg in window function
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

SELECT *, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- Test 3: Mix custom and standard aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, 
       COUNT(*),
       pdb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category, 
       COUNT(*),
       pdb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 4: Custom agg with FILTER (extracted at planning time)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) 
       FILTER (WHERE status_code >= 500)
FROM logs
WHERE description @@@ 'error';

SELECT pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) 
       FILTER (WHERE status_code >= 500)
FROM logs
WHERE description @@@ 'error';

-- Test 5: Custom agg with FILTER and OVER (window function)
-- NOTE: FILTER with window functions is currently not supported
-- This test documents the current limitation
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, pdb.agg('{"terms": {"field": "category"}}'::jsonb) 
       FILTER (WHERE status_code >= 500) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- This query is expected to fail because FILTER with OVER is not yet supported
-- The error message guides users to file an issue or use paradedb.all()
SELECT *, pdb.agg('{"terms": {"field": "category"}}'::jsonb) 
       FILTER (WHERE status_code >= 500) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- Test 6: EXPLAIN query to show custom agg is recognized
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, 
       COUNT(*),
       pdb.agg('{"max": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category, 
       COUNT(*),
       pdb.agg('{"max": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 7: pdb.agg() without @@@ operator (no WHERE clause)
-- This tests that pdb.agg() is intercepted even without search operator
-- The custom scan is now used because we detect window aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
ORDER BY timestamp DESC LIMIT 10;

-- Execute the query - should work now with custom scan
SELECT *, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
ORDER BY timestamp DESC LIMIT 10;

-- Test 8: pdb.agg() with simple WHERE condition (not @@@)
-- This tests that pdb.agg() works with regular WHERE conditions
-- The custom scan should be used because we have window aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
WHERE status_code >= 500
ORDER BY timestamp DESC LIMIT 10;

-- Execute the query - should work with custom scan
SELECT *, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
WHERE status_code >= 500
ORDER BY timestamp DESC LIMIT 10;

-- Test 9: Error handling - invalid JSON with 'buckets' wrapper (should fail fast)
SELECT *, pdb.agg('{"buckets": {"terms": {"field": "category"}}}'::jsonb) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- Test 10: Error handling - non-object JSON (should fail fast)
SELECT *, pdb.agg('"invalid"'::jsonb) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- Test 11: Error handling - invalid aggregation type (should fail fast)
SELECT *, pdb.agg('{"invalid_agg_type": {"field": "category"}}'::jsonb) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- Test 12: Error handling - pdb.agg() with FILTER clause (should fail at planner hook)
SELECT *, pdb.agg('{"terms": {"field": "category"}}'::jsonb) FILTER (WHERE status_code >= 500) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- Cleanup
DROP TABLE logs CASCADE;


