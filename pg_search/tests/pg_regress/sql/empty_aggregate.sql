-- =====================================================================
-- Empty Table Aggregate Tests
-- =====================================================================
-- This file tests aggregate functions on empty tables to ensure they
-- don't error and return expected results. Tests both SQL aggregates
-- (with enable_aggregate_custom_scan) and JSON aggregates.
-- Related to issue #2996

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

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
USING bm25 (id, value, category, price, created_at)
WITH (
    key_field='id',
    numeric_fields='{"value": {"fast": true}, "price": {"fast": true}}',
    text_fields='{"category": {"fast": true, "tokenizer": {"type": "raw", "lowercase": true}}}',
    json_fields='{}'
);

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
USING bm25 (id, message, country, severity, timestamp, metadata) 
WITH (
    key_field = 'id', 
    text_fields = '{"country": {"fast": true, "tokenizer": {"type": "raw", "lowercase": true}}}', 
    numeric_fields = '{"severity": {"fast": true}}',
    json_fields = '{"metadata": {"fast": true, "tokenizer": {"type": "raw", "lowercase": true}}}'
);

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
-- SECTION 4: JSON Aggregates on Empty Table
-- =====================================================================

-- Test 4.1: COUNT JSON aggregate
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "value_count": {
            "value_count": {"field": "value"}
        }
    }'
);

-- Test 4.2: SUM JSON aggregate
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "value_sum": {
            "sum": {"field": "value"}
        }
    }'
);

-- Test 4.3: AVG JSON aggregate
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "value_avg": {
            "avg": {"field": "value"}
        }
    }'
);

-- Test 4.4: MIN JSON aggregate
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "value_min": {
            "min": {"field": "value"}
        }
    }'
);

-- Test 4.5: MAX JSON aggregate
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "value_max": {
            "max": {"field": "value"}
        }
    }'
);

-- Test 4.6: STATS JSON aggregate (multiple stats at once)
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "value_stats": {
            "stats": {"field": "value"}
        }
    }'
);

-- Test 4.7: Multiple aggregates in single JSON
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "count_all": {
            "value_count": {"field": "value"}
        },
        "sum_all": {
            "sum": {"field": "value"}
        },
        "avg_all": {
            "avg": {"field": "value"}
        }
    }'
);

-- =====================================================================
-- SECTION 5: JSON Bucket (GROUP BY) Aggregates on Empty Table
-- =====================================================================

-- Test 5.1: Terms aggregation (GROUP BY equivalent)
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "categories": {
            "terms": {"field": "category", "size": 10}
        }
    }'
);

-- Test 5.2: Terms with sub-aggregations
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "categories": {
            "terms": {"field": "category"},
            "aggs": {
                "avg_value": {"avg": {"field": "value"}},
                "count": {"value_count": {"field": "value"}}
            }
        }
    }'
);

-- Test 5.3: Histogram aggregation
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "value_histogram": {
            "histogram": {"field": "value", "interval": 10}
        }
    }'
);

-- Test 5.4: Range aggregation
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "value_ranges": {
            "range": {
                "field": "value",
                "ranges": [
                    {"to": 50},
                    {"from": 50, "to": 100},
                    {"from": 100}
                ]
            }
        }
    }'
);

-- Test 5.5: Nested bucket aggregations
SELECT * FROM paradedb.aggregate(
    'empty_test_idx',
    paradedb.all(),
    '{
        "categories": {
            "terms": {"field": "category"},
            "aggs": {
                "value_ranges": {
                    "range": {
                        "field": "value",
                        "ranges": [
                            {"to": 50},
                            {"from": 50}
                        ]
                    },
                    "aggs": {
                        "avg_in_range": {"avg": {"field": "value"}}
                    }
                }
            }
        }
    }'
);

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
