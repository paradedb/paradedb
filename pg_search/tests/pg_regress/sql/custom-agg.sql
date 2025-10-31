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
    -- Database errors
    ('Database connection error', 'error', 'database', 150, 500, '2024-01-01 10:00:00'),
    ('Invalid query syntax error', 'error', 'database', 50, 400, '2024-01-01 10:03:00'),
    ('Database timeout error', 'critical', 'database', 3000, 503, '2024-01-01 10:05:00'),
    ('Database deadlock detected', 'error', 'database', 200, 500, '2024-01-01 10:10:00'),
    ('Database connection pool exhausted', 'critical', 'database', 5000, 503, '2024-01-01 10:15:00'),
    ('Slow database query', 'warning', 'database', 2500, 200, '2024-01-01 10:20:00'),
    
    -- API errors
    ('Failed to fetch data', 'error', 'api', 200, 404, '2024-01-01 10:01:00'),
    ('API rate limit exceeded', 'warning', 'api', 100, 429, '2024-01-01 10:06:00'),
    ('API authentication failed', 'error', 'api', 80, 401, '2024-01-01 10:11:00'),
    ('API endpoint not found', 'error', 'api', 50, 404, '2024-01-01 10:16:00'),
    ('API internal server error', 'critical', 'api', 1500, 500, '2024-01-01 10:21:00'),
    
    -- Network errors
    ('Timeout connecting to service', 'error', 'network', 5000, 503, '2024-01-01 10:02:00'),
    ('Network connection refused', 'error', 'network', 100, 503, '2024-01-01 10:07:00'),
    ('DNS resolution failed', 'error', 'network', 30, 503, '2024-01-01 10:12:00'),
    ('Network timeout error', 'critical', 'network', 10000, 504, '2024-01-01 10:17:00'),
    
    -- Application errors
    ('Application crashed', 'critical', 'application', 0, 500, '2024-01-01 10:04:00'),
    ('Memory allocation error', 'critical', 'application', 10, 500, '2024-01-01 10:08:00'),
    ('Null pointer exception', 'error', 'application', 5, 500, '2024-01-01 10:13:00'),
    ('Stack overflow error', 'critical', 'application', 2, 500, '2024-01-01 10:18:00'),
    
    -- Security errors
    ('Unauthorized access attempt', 'warning', 'security', 20, 403, '2024-01-01 10:09:00'),
    ('Invalid authentication token', 'error', 'security', 15, 401, '2024-01-01 10:14:00'),
    ('Suspicious activity detected', 'critical', 'security', 25, 403, '2024-01-01 10:19:00');

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

-- =====================================================================
-- SECTION 2: pdb.agg() with Different Aggregation Types (GROUP BY)
-- =====================================================================

-- Test 13: pdb.agg() with range aggregation
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, pdb.agg('{"range": {"field": "response_time", "ranges": [{"to": 100}, {"from": 100, "to": 1000}, {"from": 1000}]}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category, pdb.agg('{"range": {"field": "response_time", "ranges": [{"to": 100}, {"from": 100, "to": 1000}, {"from": 1000}]}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 14: pdb.agg() with histogram aggregation
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"histogram": {"field": "response_time", "interval": 100}}'::jsonb)
FROM logs
WHERE description @@@ 'error';

SELECT pdb.agg('{"histogram": {"field": "response_time", "interval": 100}}'::jsonb)
FROM logs
WHERE description @@@ 'error';

-- Test 15: pdb.agg() with stats aggregation (multiple metrics)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, pdb.agg('{"stats": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category, pdb.agg('{"stats": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 16: pdb.agg() with min aggregation
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"min": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error';

SELECT pdb.agg('{"min": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error';

-- Test 17: pdb.agg() with max aggregation
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"max": {"field": "status_code"}}'::jsonb)
FROM logs
WHERE description @@@ 'error';

SELECT pdb.agg('{"max": {"field": "status_code"}}'::jsonb)
FROM logs
WHERE description @@@ 'error';

-- Test 18: pdb.agg() with value_count aggregation
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, pdb.agg('{"value_count": {"field": "status_code"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category, pdb.agg('{"value_count": {"field": "status_code"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- =====================================================================
-- SECTION 3: Multiple pdb.agg() Calls in Same Query
-- =====================================================================

-- Test 19: Multiple pdb.agg() with different aggregation types (GROUP BY)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) AS avg_response,
       pdb.agg('{"terms": {"field": "severity"}}'::jsonb) AS severity_breakdown
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) AS avg_response,
       pdb.agg('{"terms": {"field": "severity"}}'::jsonb) AS severity_breakdown
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 20: Multiple pdb.agg() without GROUP BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) AS avg_response,
       pdb.agg('{"max": {"field": "status_code"}}'::jsonb) AS max_status
FROM logs
WHERE description @@@ 'error';

SELECT pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) AS avg_response,
       pdb.agg('{"max": {"field": "status_code"}}'::jsonb) AS max_status
FROM logs
WHERE description @@@ 'error';

-- Test 21: Mix of standard aggregates and multiple pdb.agg()
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category,
       COUNT(*) AS total_count,
       SUM(response_time) AS total_response_time,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) AS avg_response,
       pdb.agg('{"terms": {"field": "severity"}}'::jsonb) AS severity_breakdown
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category,
       COUNT(*) AS total_count,
       SUM(response_time) AS total_response_time,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) AS avg_response,
       pdb.agg('{"terms": {"field": "severity"}}'::jsonb) AS severity_breakdown
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- =====================================================================
-- SECTION 4: pdb.agg() with Complex WHERE Clauses
-- =====================================================================

-- Test 22: pdb.agg() with boolean AND in WHERE
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM logs
WHERE description @@@ 'error' AND status_code >= 500;

SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM logs
WHERE description @@@ 'error' AND status_code >= 500;

-- Test 23: pdb.agg() with boolean OR in WHERE
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error' OR description @@@ 'timeout';

SELECT pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error' OR description @@@ 'timeout';

-- Test 24: pdb.agg() with nested boolean expressions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, pdb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE (description @@@ 'error' AND status_code >= 500) OR (description @@@ 'timeout' AND response_time > 1000)
GROUP BY category;

SELECT category, pdb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE (description @@@ 'error' AND status_code >= 500) OR (description @@@ 'timeout' AND response_time > 1000)
GROUP BY category;

-- =====================================================================
-- SECTION 5: pdb.agg() with Empty Results
-- =====================================================================

-- Test 25: pdb.agg() with no matching documents
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM logs
WHERE description @@@ 'nonexistent_term_xyz';

SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM logs
WHERE description @@@ 'nonexistent_term_xyz';

-- Test 26: pdb.agg() with GROUP BY and no matching documents
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'nonexistent_term_xyz'
GROUP BY category;

SELECT category, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'nonexistent_term_xyz'
GROUP BY category;

-- =====================================================================
-- SECTION 6: pdb.agg() with Multiple GROUP BY Columns
-- =====================================================================

-- Test 27: pdb.agg() with two GROUP BY columns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, severity, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category, severity
ORDER BY category, severity;

SELECT category, severity, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category, severity
ORDER BY category, severity;

-- Test 28: pdb.agg() with GROUP BY in different column order
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "severity"}}'::jsonb), category
FROM logs
WHERE description @@@ 'error'
GROUP BY category
ORDER BY category;

SELECT pdb.agg('{"terms": {"field": "severity"}}'::jsonb), category
FROM logs
WHERE description @@@ 'error'
GROUP BY category
ORDER BY category;

-- =====================================================================
-- SECTION 7: pdb.agg() Window Functions (TopN)
-- =====================================================================

-- Test 29: Multiple pdb.agg() window functions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER () AS avg_response,
       pdb.agg('{"max": {"field": "status_code"}}'::jsonb) OVER () AS max_status
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

SELECT *,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER () AS avg_response,
       pdb.agg('{"max": {"field": "status_code"}}'::jsonb) OVER () AS max_status
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- Test 30: pdb.agg() window function with standard aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *,
       COUNT(*) OVER () AS total_count,
       pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () AS category_breakdown
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

SELECT *,
       COUNT(*) OVER () AS total_count,
       pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () AS category_breakdown
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- Test 31: pdb.agg() window function with different ORDER BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER () AS avg_response
FROM logs
WHERE description @@@ 'error'
ORDER BY response_time DESC LIMIT 5;

SELECT *,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) OVER () AS avg_response
FROM logs
WHERE description @@@ 'error'
ORDER BY response_time DESC LIMIT 5;

-- =====================================================================
-- SECTION 8: pdb.agg() with ORDER BY
-- =====================================================================

-- Test 32: pdb.agg() with ORDER BY on GROUP BY column
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category
ORDER BY category DESC;

SELECT category, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category
ORDER BY category DESC;

-- Test 33: pdb.agg() with multiple ORDER BY columns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, severity, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category, severity
ORDER BY category ASC, severity DESC;

SELECT category, severity, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category, severity
ORDER BY category ASC, severity DESC;

-- =====================================================================
-- SECTION 9: pdb.agg() with FILTER (GROUP BY context)
-- =====================================================================

-- Test 34: pdb.agg() with FILTER on indexed field (GROUP BY)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) FILTER (WHERE severity @@@ 'error')
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) FILTER (WHERE severity @@@ 'error')
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 35: pdb.agg() with FILTER on numeric field (GROUP BY)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category,
       pdb.agg('{"terms": {"field": "severity"}}'::jsonb) FILTER (WHERE status_code >= 500)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category,
       pdb.agg('{"terms": {"field": "severity"}}'::jsonb) FILTER (WHERE status_code >= 500)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Test 36: Multiple pdb.agg() with different FILTER clauses
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) FILTER (WHERE status_code >= 500) AS avg_5xx,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) FILTER (WHERE status_code < 500) AS avg_4xx
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

SELECT category,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) FILTER (WHERE status_code >= 500) AS avg_5xx,
       pdb.agg('{"avg": {"field": "response_time"}}'::jsonb) FILTER (WHERE status_code < 500) AS avg_4xx
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- =====================================================================
-- SECTION 10: pdb.agg() Edge Cases
-- =====================================================================

-- Test 37: pdb.agg() with contradictory WHERE clauses
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM logs
WHERE (description @@@ 'error') AND (NOT (description @@@ 'error'));

SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM logs
WHERE (description @@@ 'error') AND (NOT (description @@@ 'error'));

-- Test 38: pdb.agg() with tautological WHERE clause
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE (description @@@ 'error') OR (NOT (description @@@ 'error'));

SELECT pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE (description @@@ 'error') OR (NOT (description @@@ 'error'));

-- Test 39: pdb.agg() with all() query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM logs
WHERE id @@@ paradedb.all();

SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM logs
WHERE id @@@ paradedb.all();

-- Test 40: pdb.agg() with GROUP BY and all() query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE id @@@ paradedb.all()
GROUP BY category
ORDER BY category;

SELECT category, pdb.agg('{"avg": {"field": "response_time"}}'::jsonb)
FROM logs
WHERE id @@@ paradedb.all()
GROUP BY category
ORDER BY category;

-- =====================================================================
-- SECTION 11: Range Histogram (Classic Faceting Pattern)
-- =====================================================================

-- Test 41: Range histogram for response time buckets
-- This is a common faceting pattern - much more efficient than using CASE/CTE
-- Equivalent to: CASE WHEN response_time < 100 THEN '0-100' WHEN ... END
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"range": {"field": "response_time", "ranges": [
    {"to": 100, "key": "fast"},
    {"from": 100, "to": 1000, "key": "medium"},
    {"from": 1000, "key": "slow"}
]}}'::jsonb) AS response_time_buckets
FROM logs
WHERE description @@@ 'error';

SELECT pdb.agg('{"range": {"field": "response_time", "ranges": [
    {"to": 100, "key": "fast"},
    {"from": 100, "to": 1000, "key": "medium"},
    {"from": 1000, "key": "slow"}
]}}'::jsonb) AS response_time_buckets
FROM logs
WHERE description @@@ 'error';

-- Test 42: Range histogram with GROUP BY
-- Facet response time buckets per category
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, 
       pdb.agg('{"range": {"field": "response_time", "ranges": [
           {"to": 100, "key": "fast"},
           {"from": 100, "to": 1000, "key": "medium"},
           {"from": 1000, "key": "slow"}
       ]}}'::jsonb) AS response_time_buckets
FROM logs
WHERE description @@@ 'error'
GROUP BY category
ORDER BY category;

SELECT category, 
       pdb.agg('{"range": {"field": "response_time", "ranges": [
           {"to": 100, "key": "fast"},
           {"from": 100, "to": 1000, "key": "medium"},
           {"from": 1000, "key": "slow"}
       ]}}'::jsonb) AS response_time_buckets
FROM logs
WHERE description @@@ 'error'
GROUP BY category
ORDER BY category;

-- Test 43: Range histogram for status codes (HTTP status buckets)
-- Common pattern: 2xx, 3xx, 4xx, 5xx buckets
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"range": {"field": "status_code", "ranges": [
    {"from": 200, "to": 300, "key": "2xx"},
    {"from": 300, "to": 400, "key": "3xx"},
    {"from": 400, "to": 500, "key": "4xx"},
    {"from": 500, "to": 600, "key": "5xx"}
]}}'::jsonb) AS status_code_buckets
FROM logs
WHERE id @@@ paradedb.all();

SELECT pdb.agg('{"range": {"field": "status_code", "ranges": [
    {"from": 200, "to": 300, "key": "2xx"},
    {"from": 300, "to": 400, "key": "3xx"},
    {"from": 400, "to": 500, "key": "4xx"},
    {"from": 500, "to": 600, "key": "5xx"}
]}}'::jsonb) AS status_code_buckets
FROM logs
WHERE id @@@ paradedb.all();

-- Test 44: Multiple range histograms in one query
-- Get both response time and status code distributions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"range": {"field": "response_time", "ranges": [
           {"to": 100, "key": "fast"},
           {"from": 100, "to": 1000, "key": "medium"},
           {"from": 1000, "key": "slow"}
       ]}}'::jsonb) AS response_time_buckets,
       pdb.agg('{"range": {"field": "status_code", "ranges": [
           {"from": 400, "to": 500, "key": "4xx"},
           {"from": 500, "to": 600, "key": "5xx"}
       ]}}'::jsonb) AS status_code_buckets
FROM logs
WHERE description @@@ 'error';

SELECT pdb.agg('{"range": {"field": "response_time", "ranges": [
           {"to": 100, "key": "fast"},
           {"from": 100, "to": 1000, "key": "medium"},
           {"from": 1000, "key": "slow"}
       ]}}'::jsonb) AS response_time_buckets,
       pdb.agg('{"range": {"field": "status_code", "ranges": [
           {"from": 400, "to": 500, "key": "4xx"},
           {"from": 500, "to": 600, "key": "5xx"}
       ]}}'::jsonb) AS status_code_buckets
FROM logs
WHERE description @@@ 'error';

-- Test 45: Range histogram with TopN (window function)
-- Get response time distribution alongside top N results
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *,
       pdb.agg('{"range": {"field": "response_time", "ranges": [
           {"to": 100, "key": "fast"},
           {"from": 100, "to": 1000, "key": "medium"},
           {"from": 1000, "key": "slow"}
       ]}}'::jsonb) OVER () AS response_time_distribution
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

SELECT *,
       pdb.agg('{"range": {"field": "response_time", "ranges": [
           {"to": 100, "key": "fast"},
           {"from": 100, "to": 1000, "key": "medium"},
           {"from": 1000, "key": "slow"}
       ]}}'::jsonb) OVER () AS response_time_distribution
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC LIMIT 10;

-- =====================================================================
-- SECTION 12: Error Handling - Aggregate Custom Scan Disabled
-- =====================================================================

-- Test 46: pdb.agg() with aggregate custom scan disabled (should error)
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM logs
WHERE description @@@ 'error';

-- Test 47: pdb.agg() with GROUP BY and aggregate custom scan disabled (should error)
SELECT category, pdb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Re-enable for cleanup
SET paradedb.enable_aggregate_custom_scan TO on;

-- Cleanup
DROP TABLE logs CASCADE;


