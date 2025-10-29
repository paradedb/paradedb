-- Test custom agg function with paradedb.agg()

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
SELECT category, paradedb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category, paradedb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 2: Custom agg in window function
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, paradedb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

SELECT *, paradedb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- Test 3: Mix custom and standard aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, 
       COUNT(*),
       paradedb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category, 
       COUNT(*),
       paradedb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 4: Custom agg with FILTER (extracted at planning time)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT paradedb.agg('{"avg": {"field": "response_time"}}'::jsonb) 
       FILTER (WHERE status_code >= 500)
FROM logs
WHERE description @@@ 'error';

SELECT paradedb.agg('{"avg": {"field": "response_time"}}'::jsonb) 
       FILTER (WHERE status_code >= 500)
FROM logs
WHERE description @@@ 'error';

-- Test 5: Custom agg with FILTER and OVER (window function)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, paradedb.agg('{"terms": {"field": "category"}}'::jsonb) 
       FILTER (WHERE status_code >= 500) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

SELECT *, paradedb.agg('{"terms": {"field": "category"}}'::jsonb) 
       FILTER (WHERE status_code >= 500) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- Test 6: EXPLAIN query to show custom agg is recognized
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, 
       COUNT(*),
       paradedb.agg('{"max": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category, 
       COUNT(*),
       paradedb.agg('{"max": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 7: paradedb.agg() without @@@ operator (no WHERE clause)
-- This tests that paradedb.agg() is intercepted even without search operator
-- The custom scan is now used because we detect window aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, paradedb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
ORDER BY timestamp DESC LIMIT 10;

-- Execute the query - should work now with custom scan
SELECT *, paradedb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
ORDER BY timestamp DESC LIMIT 10;

-- Test 8: paradedb.agg() with simple WHERE condition (not @@@)
-- This tests that paradedb.agg() works with regular WHERE conditions
-- The custom scan should be used because we have window aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, paradedb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
WHERE status_code >= 500
ORDER BY timestamp DESC LIMIT 10;

-- Execute the query - should work with custom scan
SELECT *, paradedb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER ()
FROM logs
WHERE status_code >= 500
ORDER BY timestamp DESC LIMIT 10;

-- Cleanup
DROP TABLE logs CASCADE;


