-- Test JSON field aggregates without GROUP BY (using aggregate custom scan)

-- Create extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on;

-- =========================================
-- Test 1: Simple COUNT on JSON filtered data
-- =========================================

-- Create test table
CREATE TABLE json_agg_test (
    id SERIAL PRIMARY KEY,
    metadata JSONB,
    data JSONB
);

-- Insert test data
INSERT INTO json_agg_test (metadata, data) VALUES
    ('{"category": "electronics", "brand": "Apple", "price": 999}', '{"color": "silver", "stock": 10}'),
    ('{"category": "electronics", "brand": "Samsung", "price": 799}', '{"color": "black", "stock": 15}'),
    ('{"category": "electronics", "brand": "Apple", "price": 1299}', '{"color": "gold", "stock": 5}'),
    ('{"category": "clothing", "brand": "Nike", "price": 89}', '{"size": "M", "stock": 20}'),
    ('{"category": "clothing", "brand": "Adidas", "price": 79}', '{"size": "L", "stock": 25}'),
    ('{"category": "clothing", "brand": "Nike", "price": 99}', '{"size": "S", "stock": 30}'),
    ('{"category": "home", "brand": "Ikea", "price": 199}', '{"material": "wood", "stock": 8}'),
    ('{"category": "home", "brand": "HomeDepot", "price": 299}', '{"material": "metal", "stock": 12}');

-- Create BM25 index
CREATE INDEX idx_json_agg ON json_agg_test
USING bm25 (id, metadata, data)
WITH (
    key_field = 'id',
    json_fields = '{
        "metadata": {"indexed": true, "fast": true, "expand_dots": true},
        "data": {"indexed": true, "fast": true, "expand_dots": true}
    }'
);

-- Test simple COUNT with JSON field filter with EXPLAIN
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.category');

-- Execute the query
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.category');

-- =========================================
-- Test 2: COUNT with specific JSON value filter
-- =========================================

-- Test COUNT with specific category filter with EXPLAIN
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.phrase_prefix('metadata.category', 'elect');

-- Execute the query
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.phrase_prefix('metadata.category', 'elect');

-- =========================================
-- Test 3: COUNT with multiple JSON field filters
-- =========================================

-- Test COUNT with multiple JSON field conditions with EXPLAIN
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.category') 
  AND id @@@ paradedb.exists('metadata.brand');

-- Execute the query
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.category') 
  AND id @@@ paradedb.exists('metadata.brand');

-- =========================================
-- Test 4: COUNT with complex JSON filters
-- =========================================

-- Test COUNT with term query on JSON field with EXPLAIN
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.brand', 'Apple');

-- Execute the query
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.brand', 'Apple');

-- =========================================
-- Test 5: COUNT with range queries on JSON fields
-- =========================================

-- Test COUNT with range query on nested JSON data with EXPLAIN
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.range('data.stock', gte => 10, lte => 20);

-- Execute the query
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.range('data.stock', gte => 10, lte => 20);

-- =========================================
-- Test 6: COUNT with boolean queries on JSON
-- =========================================

-- Test COUNT with boolean must query with EXPLAIN
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.boolean(
    must => ARRAY[
        paradedb.exists('metadata.category'),
        paradedb.term('metadata.category', 'electronics')
    ]
);

-- Execute the query
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.boolean(
    must => ARRAY[
        paradedb.exists('metadata.category'),
        paradedb.term('metadata.category', 'electronics')
    ]
);

-- =========================================
-- Test 7: Verify aggregate custom scan is used
-- =========================================

-- Verify that these queries use the ParadeDB Aggregate Scan
-- by checking EXPLAIN output contains "ParadeDB Aggregate Scan"

-- This should show ParadeDB Aggregate Scan
EXPLAIN (COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.category');

-- This should also show ParadeDB Aggregate Scan  
EXPLAIN (COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.brand', 'Apple');

-- =========================================
-- Test 8: Compare with regular PostgreSQL aggregates
-- =========================================

-- Disable aggregate custom scan to compare
SET paradedb.enable_aggregate_custom_scan TO off;

-- This should show regular PostgreSQL Aggregate + Custom Scan
EXPLAIN (COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.category');

-- Re-enable for consistency
SET paradedb.enable_aggregate_custom_scan TO on;

-- =========================================
-- Test 9: Edge cases with JSON aggregates
-- =========================================

-- Test COUNT on empty result set
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.nonexistent', 'value');

-- Test COUNT with all() query
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.all();

-- Clean up
DROP TABLE json_agg_test;
