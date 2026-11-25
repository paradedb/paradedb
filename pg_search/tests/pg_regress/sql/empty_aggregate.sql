-- =====================================================================
-- Empty Table Aggregate Tests
-- =====================================================================
-- This file tests aggregate functions on empty tables to ensure they
-- don't error and return expected results. Tests both SQL aggregates
-- (with enable_aggregate_custom_scan) and JSON aggregates.
-- Related to issue #2996

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.add_doc_count_to_aggs TO on;

-- =====================================================================
-- SECTION 1: Setup Empty Tables
-- =====================================================================

DROP TABLE IF EXISTS empty_test CASCADE;
CREATE TABLE empty_test (
    id SERIAL PRIMARY KEY,
    value INTEGER,
    category TEXT,
    price NUMERIC,
    created_at TIMESTAMP
);

-- Create index with fast fields for aggregation
CREATE INDEX empty_test_idx ON empty_test
USING paradedb (id, value, (category::pdb.literal_normalized('lowercase=true')), price, created_at);

-- Create a second empty table for additional tests
DROP TABLE IF EXISTS empty_logs CASCADE;
CREATE TABLE empty_logs (
    id SERIAL PRIMARY KEY,
    message TEXT,
    country VARCHAR(255),
    severity INTEGER,
    timestamp TIMESTAMP,
    metadata JSONB
);

CREATE INDEX empty_logs_idx ON empty_logs
USING paradedb (id, message, (country::pdb.literal_normalized('lowercase=true')), severity, timestamp, (metadata::pdb.literal_normalized('lowercase=true')));

-- =====================================================================
-- SECTION 2: Simple SQL Aggregates on Empty Table
-- =====================================================================

-- Test 2.1: COUNT(*) - should return 0
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM empty_test WHERE id @@@ paradedb.all();

SELECT COUNT(*) FROM empty_test WHERE id @@@ paradedb.all();

-- Test 2.2: COUNT(column) - should return 0
SELECT COUNT(value) FROM empty_test WHERE id @@@ paradedb.all();

-- Test 2.3: SUM - should return NULL
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM(value) FROM empty_test WHERE id @@@ paradedb.all();

SELECT SUM(value) FROM empty_test WHERE id @@@ paradedb.all();

-- Test 2.4: AVG - should return NULL
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG(value) FROM empty_test WHERE id @@@ paradedb.all();

SELECT AVG(value) FROM empty_test WHERE id @@@ paradedb.all();

-- Test 2.5: MIN - should return NULL
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MIN(value) FROM empty_test WHERE id @@@ paradedb.all();

SELECT MIN(value) FROM empty_test WHERE id @@@ paradedb.all();

-- Test 2.6: MAX - should return NULL
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MAX(value) FROM empty_test WHERE id @@@ paradedb.all();

SELECT MAX(value) FROM empty_test WHERE id @@@ paradedb.all();

-- Test 2.7: Multiple aggregates in single query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), COUNT(value), SUM(value), AVG(value), MIN(value), MAX(value)
FROM empty_test WHERE id @@@ paradedb.all();

SELECT COUNT(*), COUNT(value), SUM(value), AVG(value), MIN(value), MAX(value)
FROM empty_test WHERE id @@@ paradedb.all();

-- Test 2.8: From the original issue report
SELECT COUNT(*) FROM empty_logs WHERE id @@@ paradedb.all();

-- =====================================================================
-- SECTION 3: GROUP BY SQL Aggregates on Empty Table
-- =====================================================================

-- Test 3.1: Simple GROUP BY - should return empty result set (0 rows)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) FROM empty_test
WHERE id @@@ paradedb.all()
GROUP BY category;

SELECT category, COUNT(*) FROM empty_test
WHERE id @@@ paradedb.all()
GROUP BY category;

-- Test 3.2: GROUP BY with ORDER BY - should return empty result set
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*), SUM(value), AVG(value)
FROM empty_test
WHERE id @@@ paradedb.all()
GROUP BY category
ORDER BY category;

SELECT category, COUNT(*), SUM(value), AVG(value)
FROM empty_test
WHERE id @@@ paradedb.all()
GROUP BY category
ORDER BY category;

-- Test 3.3: GROUP BY with LIMIT - should return empty result set
SELECT category, COUNT(*) FROM empty_test
WHERE id @@@ paradedb.all()
GROUP BY category
LIMIT 10;

-- Test 3.4: Multiple GROUP BY columns - should return empty result set
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, value, COUNT(*) FROM empty_test
WHERE id @@@ paradedb.all()
GROUP BY category, value
ORDER BY category, value;

SELECT category, value, COUNT(*) FROM empty_test
WHERE id @@@ paradedb.all()
GROUP BY category, value
ORDER BY category, value;

-- Test 3.5: From the original issue report
SELECT severity, COUNT(*) FROM empty_logs
WHERE id @@@ paradedb.all()
GROUP BY severity
ORDER BY severity DESC
LIMIT 10;

-- =====================================================================
-- SECTION 6: Edge Cases and Complex Queries on Empty Tables
-- =====================================================================

-- Test 6.1: HAVING clause with empty GROUP BY result
SELECT category, COUNT(*) as cnt
FROM empty_test
WHERE id @@@ paradedb.all()
GROUP BY category
HAVING COUNT(*) > 0;

-- Test 6.2: Complex aggregates with expressions
SELECT
    category,
    COUNT(*) as total_count,
    COUNT(DISTINCT value) as distinct_values,
    SUM(CASE WHEN value > 50 THEN 1 ELSE 0 END) as high_values,
    AVG(value * 2) as doubled_avg
FROM empty_test
WHERE id @@@ paradedb.all()
GROUP BY category;

-- Test 6.3: Aggregates with FILTER clause
SELECT
    COUNT(*) FILTER (WHERE value > 50) as high_count,
    SUM(value) FILTER (WHERE category = 'Electronics') as electronics_sum
FROM empty_test
WHERE id @@@ paradedb.all();

-- Test 6.4: Multiple tables (though both empty)
SELECT COUNT(*) FROM empty_test WHERE id @@@ paradedb.all()
UNION ALL
SELECT COUNT(*) FROM empty_logs WHERE id @@@ paradedb.all();

-- =====================================================================
-- SECTION 7: Disable aggregate custom scan - verify normal behavior
-- =====================================================================

SET paradedb.enable_aggregate_custom_scan TO off;

-- These should work normally (no custom scan, standard PostgreSQL behavior)
SELECT COUNT(*) FROM empty_test WHERE id @@@ paradedb.all();
SELECT category, COUNT(*) FROM empty_test WHERE id @@@ paradedb.all() GROUP BY category;

-- Re-enable for cleanup
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE IF EXISTS empty_test CASCADE;
DROP TABLE IF EXISTS empty_logs CASCADE;
