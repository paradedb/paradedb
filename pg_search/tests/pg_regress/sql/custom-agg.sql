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
-- SECTION 12: pdb.agg() with Aggregate Custom Scan GUC Disabled
-- =====================================================================

-- Test 46: pdb.agg() should work even when GUC is disabled (explicit opt-in)
SET paradedb.enable_aggregate_custom_scan TO off;

-- Should still work because pdb.agg() is an explicit opt-in
SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM logs
WHERE description @@@ 'error';

-- Test 47: pdb.agg() with GROUP BY should also work when GUC is disabled
SELECT category, pdb.agg('{"terms": {"field": "severity"}}'::jsonb)
FROM logs
WHERE description @@@ 'error'
GROUP BY category;

-- Re-enable for cleanup
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 48: pdb.agg() as window function with WHERE clause when filter_pushdown is disabled
-- The custom scan will reject queries where quals can't be extracted
-- Result: Query executes correctly via PostgreSQL (not custom scan), returns filtered results
-- This is SAFE behavior - WHERE clause is applied correctly, no silent data loss
SET paradedb.enable_filter_pushdown TO off;
EXPLAIN SELECT id, description, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM logs
WHERE response_time = 150
ORDER BY timestamp DESC
LIMIT 1;

SELECT id, description, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM logs
WHERE response_time = 150
ORDER BY timestamp DESC
LIMIT 1;

-- Test 49: pdb.agg() with exact text match WHERE clause and filter_pushdown enabled
-- With filter_pushdown ON, the custom scan should handle this via Qual::All + PostgreSQL filtering
-- Result: Custom scan uses Qual::All, PostgreSQL applies WHERE clause filter, returns correct results
SET paradedb.enable_filter_pushdown TO on;
EXPLAIN SELECT id, description, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM logs
WHERE description = 'Database connection error'
ORDER BY timestamp DESC
LIMIT 1;

SELECT id, description, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM logs
WHERE description = 'Database connection error'
ORDER BY timestamp DESC
LIMIT 1;
SET paradedb.enable_filter_pushdown TO off;

-- Test 50: pdb.agg() as window function with no WHERE clause should work
EXPLAIN SELECT id, description, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM logs
ORDER BY timestamp DESC
LIMIT 1;

SELECT id, description, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM logs
ORDER BY timestamp DESC
LIMIT 1;

-- Test 51: pdb.agg() with @@@ AND non-indexed field equality (filter_pushdown OFF)
-- This demonstrates the limitation: mixing @@@ with non-pushable predicates will error
-- when filter_pushdown is disabled
SET paradedb.enable_filter_pushdown TO off;
SELECT id, description, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM logs
WHERE description @@@ 'error' AND description = 'Database connection error'
ORDER BY timestamp DESC
LIMIT 1;

-- Test 52: Same query works with filter_pushdown enabled
SET paradedb.enable_filter_pushdown TO on;
SELECT id, description, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM logs
WHERE description @@@ 'error' AND description = 'Database connection error'
ORDER BY timestamp DESC
LIMIT 1;
SET paradedb.enable_filter_pushdown TO off;

-- Test 51: pdb.agg() as window function with @@@ WHERE clause
-- Currently requires filter_pushdown because we can't determine at planner time
-- if ALL predicates are pushable (conservative approach to prevent silent data loss)
SET paradedb.enable_filter_pushdown TO on;
SELECT id, description, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM logs
WHERE description @@@ 'error'
ORDER BY timestamp DESC
LIMIT 1;

-- Cleanup
DROP TABLE logs CASCADE;
RESET paradedb.enable_filter_pushdown;

-- =====================================================================
-- SECTION 13: Nested Aggregations with pdb.agg()
-- =====================================================================
-- Tests for nested aggregations using "aggs" field in JSON
-- This demonstrates the difference between:
-- 1. Using GROUP BY with multiple columns (creates nested terms aggregations)
-- 2. Using pdb.agg() with nested "aggs" in JSON (creates nested structure)
-- 3. Using multiple pdb.agg() calls (creates parallel aggregations)

DROP TABLE IF EXISTS products CASCADE;
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    brand TEXT,
    rating INTEGER,
    price NUMERIC
);

INSERT INTO products (description, category, brand, rating, price) VALUES
    ('Laptop with fast processor', 'Electronics', 'Apple', 5, 1299.99),
    ('Gaming laptop with RGB', 'Electronics', 'Dell', 5, 1499.99),
    ('Budget laptop', 'Electronics', 'HP', 3, 499.99),
    ('Wireless keyboard', 'Electronics', 'Logitech', 4, 79.99),
    ('Mechanical keyboard', 'Electronics', 'Corsair', 5, 149.99),
    ('Running shoes', 'Sports', 'Nike', 5, 89.99),
    ('Basketball shoes', 'Sports', 'Adidas', 4, 119.99),
    ('Winter jacket', 'Clothing', 'North Face', 4, 199.99),
    ('Summer jacket', 'Clothing', 'Patagonia', 3, 129.99),
    ('Toy laptop', 'Toys', 'Fisher Price', 3, 29.99);

CREATE INDEX products_idx ON products
USING bm25 (id, description, category, brand, rating, price)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}, "brand": {"fast": true}}',
    numeric_fields='{"rating": {"fast": true}, "price": {"fast": true}}'
);

-- Test 52: GROUP BY with two columns creates NESTED terms aggregations
-- This groups first by category, then within each category bucket, groups by brand
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, brand, COUNT(*), AVG(price)
FROM products
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category, brand
ORDER BY category, brand;

SELECT category, brand, COUNT(*), AVG(price)
FROM products
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category, brand
ORDER BY category, brand;

-- Test 53: GROUP BY with three columns creates TRIPLE-NESTED terms aggregations
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, brand, rating, COUNT(*), AVG(price)
FROM products
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category, brand, rating
ORDER BY category, brand, rating;

SELECT category, brand, rating, COUNT(*), AVG(price)
FROM products
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category, brand, rating
ORDER BY category, brand, rating;

-- Test 54: Using pdb.agg() with nested terms (equivalent to GROUP BY category, brand)
-- This should produce the same nested structure as Test 52
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "category", "aggs": {"brand_breakdown": {"terms": {"field": "brand"}}}}}'::jsonb)
FROM products
WHERE description @@@ 'laptop OR keyboard';

SELECT pdb.agg('{"terms": {"field": "category", "aggs": {"brand_breakdown": {"terms": {"field": "brand"}}}}}'::jsonb)
FROM products
WHERE description @@@ 'laptop OR keyboard';

-- Test 55: Using pdb.agg() with triple-nested terms
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "category", "aggs": {"brand_breakdown": {"terms": {"field": "brand", "aggs": {"rating_breakdown": {"terms": {"field": "rating"}}}}}}}}'::jsonb)
FROM products
WHERE description @@@ 'laptop OR keyboard';

SELECT pdb.agg('{"terms": {"field": "category", "aggs": {"brand_breakdown": {"terms": {"field": "brand", "aggs": {"rating_breakdown": {"terms": {"field": "rating"}}}}}}}}'::jsonb)
FROM products
WHERE description @@@ 'laptop OR keyboard';

-- Test 56: Multiple pdb.agg() calls with one term each
-- These run as SEPARATE, INDEPENDENT aggregations (not nested)
-- Each pdb.agg() returns its own complete breakdown
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    pdb.agg('{"terms": {"field": "category"}}'::jsonb) AS category_breakdown,
    pdb.agg('{"terms": {"field": "brand"}}'::jsonb) AS brand_breakdown
FROM products
WHERE description @@@ 'laptop OR keyboard';

SELECT 
    pdb.agg('{"terms": {"field": "category"}}'::jsonb) AS category_breakdown,
    pdb.agg('{"terms": {"field": "brand"}}'::jsonb) AS brand_breakdown
FROM products
WHERE description @@@ 'laptop OR keyboard';

-- Test 57: Multiple pdb.agg() calls with different aggregation types
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    pdb.agg('{"terms": {"field": "category"}}'::jsonb) AS category_breakdown,
    pdb.agg('{"terms": {"field": "brand"}}'::jsonb) AS brand_breakdown,
    pdb.agg('{"avg": {"field": "price"}}'::jsonb) AS avg_price
FROM products
WHERE description @@@ 'laptop OR keyboard';

SELECT 
    pdb.agg('{"terms": {"field": "category"}}'::jsonb) AS category_breakdown,
    pdb.agg('{"terms": {"field": "brand"}}'::jsonb) AS brand_breakdown,
    pdb.agg('{"avg": {"field": "price"}}'::jsonb) AS avg_price
FROM products
WHERE description @@@ 'laptop OR keyboard';

-- Test 58: One GROUP BY column with pdb.agg() for sub-aggregation
-- This groups by category, and within each category, gets brand breakdown
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    category,
    pdb.agg('{"terms": {"field": "brand"}}'::jsonb) AS brand_breakdown,
    COUNT(*) AS count
FROM products
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category
ORDER BY category;

SELECT 
    category,
    pdb.agg('{"terms": {"field": "brand"}}'::jsonb) AS brand_breakdown,
    COUNT(*) AS count
FROM products
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category
ORDER BY category;

-- Test 59: Multiple pdb.agg() with GROUP BY
-- Each pdb.agg() is computed independently for each category group
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    category,
    pdb.agg('{"terms": {"field": "brand"}}'::jsonb) AS brand_breakdown,
    pdb.agg('{"avg": {"field": "rating"}}'::jsonb) AS avg_rating,
    COUNT(*) AS count
FROM products
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category
ORDER BY category;

SELECT 
    category,
    pdb.agg('{"terms": {"field": "brand"}}'::jsonb) AS brand_breakdown,
    pdb.agg('{"avg": {"field": "rating"}}'::jsonb) AS avg_rating,
    COUNT(*) AS count
FROM products
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category
ORDER BY category;

-- Test 60: pdb.agg() with terms and metric sub-aggregations
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "category", "aggs": {"avg_price": {"avg": {"field": "price"}}, "max_rating": {"max": {"field": "rating"}}}}}'::jsonb)
FROM products
WHERE description @@@ 'laptop OR keyboard';

SELECT pdb.agg('{"terms": {"field": "category", "aggs": {"avg_price": {"avg": {"field": "price"}}, "max_rating": {"max": {"field": "rating"}}}}}'::jsonb)
FROM products
WHERE description @@@ 'laptop OR keyboard';

-- Test 61: Comparing GROUP BY vs pdb.agg() with same nesting
-- GROUP BY approach
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, brand, AVG(price) AS avg_price, MAX(rating) AS max_rating
FROM products
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category, brand
ORDER BY category, brand;

SELECT category, brand, AVG(price) AS avg_price, MAX(rating) AS max_rating
FROM products
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category, brand
ORDER BY category, brand;

-- pdb.agg() approach (returns JSON structure)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "category", "aggs": {"brand_breakdown": {"terms": {"field": "brand", "aggs": {"avg_price": {"avg": {"field": "price"}}, "max_rating": {"max": {"field": "rating"}}}}}}}}'::jsonb)
FROM products
WHERE description @@@ 'laptop OR keyboard';

SELECT pdb.agg('{"terms": {"field": "category", "aggs": {"brand_breakdown": {"terms": {"field": "brand", "aggs": {"avg_price": {"avg": {"field": "price"}}, "max_rating": {"max": {"field": "rating"}}}}}}}}'::jsonb)
FROM products
WHERE description @@@ 'laptop OR keyboard';

-- Test 62: Multiple independent terms vs nested terms - showing the difference
-- Independent: Each field gets its own top-level breakdown
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    pdb.agg('{"terms": {"field": "category"}}'::jsonb) AS categories,
    pdb.agg('{"terms": {"field": "brand"}}'::jsonb) AS brands,
    pdb.agg('{"terms": {"field": "rating"}}'::jsonb) AS ratings
FROM products
WHERE description @@@ 'laptop OR keyboard';

SELECT 
    pdb.agg('{"terms": {"field": "category"}}'::jsonb) AS categories,
    pdb.agg('{"terms": {"field": "brand"}}'::jsonb) AS brands,
    pdb.agg('{"terms": {"field": "rating"}}'::jsonb) AS ratings
FROM products
WHERE description @@@ 'laptop OR keyboard';

-- Nested: Shows category -> brand -> rating hierarchy
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "category", "aggs": {"brands": {"terms": {"field": "brand", "aggs": {"ratings": {"terms": {"field": "rating"}}}}}}}}'::jsonb) AS nested_breakdown
FROM products
WHERE description @@@ 'laptop OR keyboard';

SELECT pdb.agg('{"terms": {"field": "category", "aggs": {"brands": {"terms": {"field": "brand", "aggs": {"ratings": {"terms": {"field": "rating"}}}}}}}}'::jsonb) AS nested_breakdown
FROM products
WHERE description @@@ 'laptop OR keyboard';

-- Cleanup
DROP TABLE products CASCADE;

-- =====================================================================
-- SECTION 14: MVCC Visibility Toggle for pdb.agg()
-- =====================================================================
-- Tests for the optional second argument to pdb.agg() that controls
-- MVCC visibility filtering. This allows users to trade accuracy for
-- performance when exact transaction-consistent aggregates are not required.
--
-- Syntax: pdb.agg(agg_spec, solve_mvcc)
-- - solve_mvcc=true (default): Apply MVCC filtering for transaction-accurate aggregates
-- - solve_mvcc=false: Skip MVCC filtering - includes deleted docs still in the index
--
-- IMPORTANT: To see the difference, we need deleted rows that are still in the index.
-- After DELETE, the document remains in Tantivy until a merge/vacuum operation.
-- With MVCC enabled, deleted docs are filtered out.
-- With MVCC disabled, deleted docs are included in aggregates.

DROP TABLE IF EXISTS mvcc_test CASCADE;
CREATE TABLE mvcc_test (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    value INT
);

-- Insert initial data
INSERT INTO mvcc_test (description, category, value) VALUES
    ('Test item one', 'A', 100),
    ('Test item two', 'A', 200),
    ('Test item three', 'B', 300),
    ('Test item four', 'B', 400),
    ('Test item five', 'C', 500);

-- Create index BEFORE deleting - so deleted docs remain in the index
CREATE INDEX mvcc_test_idx ON mvcc_test
USING bm25 (id, description, category, value)
WITH (
    key_field = 'id',
    text_fields = '{"description": {}, "category": {"fast": true}}',
    numeric_fields = '{"value": {"fast": true}}'
);

-- Delete some rows - docs remain in Tantivy index until merge
DELETE FROM mvcc_test WHERE id IN (4, 5);

-- Now we have:
-- Visible rows: id=1 (100), id=2 (200), id=3 (300) = avg 200
-- All docs in index: id=1 (100), id=2 (200), id=3 (300), id=4 (400), id=5 (500) = avg 300

-- Test 63: pdb.agg() with MVCC enabled (default behavior)
-- This should apply MVCC filtering for accurate results
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, pdb.agg('{"avg": {"field": "value"}}'::jsonb) OVER () AS avg_value
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

SELECT *, pdb.agg('{"avg": {"field": "value"}}'::jsonb) OVER () AS avg_value
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

-- Test 64: pdb.agg() with explicit solve_mvcc = true (same as default)
-- Note: PostgreSQL aggregates don't support named arguments, so we use positional
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, pdb.agg('{"avg": {"field": "value"}}'::jsonb, true) OVER () AS avg_value
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

SELECT *, pdb.agg('{"avg": {"field": "value"}}'::jsonb, true) OVER () AS avg_value
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

-- Test 65: pdb.agg() with solve_mvcc = false for performance
-- This skips MVCC filtering for faster aggregation
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, pdb.agg('{"avg": {"field": "value"}}'::jsonb, false) OVER () AS avg_value
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

SELECT *, pdb.agg('{"avg": {"field": "value"}}'::jsonb, false) OVER () AS avg_value
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

-- Test 66: pdb.agg() with terms aggregation and solve_mvcc = false
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, pdb.agg('{"terms": {"field": "category"}}'::jsonb, false) OVER () AS category_counts
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

SELECT *, pdb.agg('{"terms": {"field": "category"}}'::jsonb, false) OVER () AS category_counts
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

-- Test 67: Multiple pdb.agg() calls with SAME solve_mvcc = false
-- When all aggregates have solve_mvcc = false, MVCC filtering is disabled
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *,
       pdb.agg('{"avg": {"field": "value"}}'::jsonb, false) OVER () AS avg_no_mvcc,
       pdb.agg('{"terms": {"field": "category"}}'::jsonb, false) OVER () AS terms_no_mvcc
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

SELECT *,
       pdb.agg('{"avg": {"field": "value"}}'::jsonb, false) OVER () AS avg_no_mvcc,
       pdb.agg('{"terms": {"field": "category"}}'::jsonb, false) OVER () AS terms_no_mvcc
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

-- Test 67b: Multiple pdb.agg() calls with CONTRADICTING solve_mvcc settings should ERROR
-- Mixing true and false is not allowed - must be consistent
SELECT *,
       pdb.agg('{"avg": {"field": "value"}}'::jsonb, true) OVER () AS avg_with_mvcc,
       pdb.agg('{"terms": {"field": "category"}}'::jsonb, false) OVER () AS terms_no_mvcc
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

-- Test 67c: Multiple pdb.agg() - default (true) mixed with explicit false should also ERROR
SELECT *,
       pdb.agg('{"avg": {"field": "value"}}'::jsonb) OVER () AS avg_default,
       pdb.agg('{"terms": {"field": "category"}}'::jsonb, false) OVER () AS terms_no_mvcc
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

-- Test 67d: Multiple pdb.agg() - all with default (true) should work fine
SELECT *,
       pdb.agg('{"avg": {"field": "value"}}'::jsonb) OVER () AS avg_default,
       pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () AS terms_default
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

-- Test 68: pdb.agg() in GROUP BY context with solve_mvcc = false should ERROR
-- solve_mvcc=false is only allowed in TopN (window function) context
SELECT category, pdb.agg('{"avg": {"field": "value"}}'::jsonb, false)
FROM mvcc_test
WHERE description @@@ 'test'
GROUP BY category;

-- Test 68b: pdb.agg() in GROUP BY context with solve_mvcc = true should work
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, pdb.agg('{"avg": {"field": "value"}}'::jsonb, true)
FROM mvcc_test
WHERE description @@@ 'test'
GROUP BY category;

SELECT category, pdb.agg('{"avg": {"field": "value"}}'::jsonb, true)
FROM mvcc_test
WHERE description @@@ 'test'
GROUP BY category;

-- Test 68c: pdb.agg() in GROUP BY context with default (no second arg) should work
SELECT category, pdb.agg('{"avg": {"field": "value"}}'::jsonb)
FROM mvcc_test
WHERE description @@@ 'test'
GROUP BY category;

-- Test 69: Complex aggregation with solve_mvcc = false
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT *, pdb.agg('{"range": {"field": "value", "ranges": [{"to": 200}, {"from": 200, "to": 400}, {"from": 400}]}}'::jsonb, false) OVER () AS value_ranges
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

SELECT *, pdb.agg('{"range": {"field": "value", "ranges": [{"to": 200}, {"from": 200, "to": 400}, {"from": 400}]}}'::jsonb, false) OVER () AS value_ranges
FROM mvcc_test
WHERE description @@@ 'test'
ORDER BY id DESC LIMIT 3;

-- Cleanup
DROP TABLE mvcc_test CASCADE;
