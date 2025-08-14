-- Test JSON field GROUP BY with aggregate custom scan

-- Create extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on;

-- =========================================
-- Test 1: Single JSON field GROUP BY
-- =========================================

-- Create test table
CREATE TABLE json_test_single (
    id SERIAL PRIMARY KEY,
    metadata JSONB,
    data JSONB
);

-- Insert test data
INSERT INTO json_test_single (metadata, data) VALUES
    ('{"category": "electronics", "brand": "Apple", "price": 999}', '{"color": "silver", "stock": 10}'),
    ('{"category": "electronics", "brand": "Samsung", "price": 799}', '{"color": "black", "stock": 15}'),
    ('{"category": "electronics", "brand": "Apple", "price": 1299}', '{"color": "gold", "stock": 5}'),
    ('{"category": "clothing", "brand": "Nike", "price": 89}', '{"size": "M", "stock": 20}'),
    ('{"category": "clothing", "brand": "Adidas", "price": 79}', '{"size": "L", "stock": 25}'),
    ('{"category": "clothing", "brand": "Nike", "price": 99}', '{"size": "S", "stock": 30}');

-- Create BM25 index
CREATE INDEX idx_json_single ON json_test_single
USING bm25 (id, metadata, data)
WITH (
    key_field = 'id',
    json_fields = '{
        "metadata": {"indexed": true, "fast": true, "expand_dots": true},
        "data": {"indexed": true, "fast": true, "expand_dots": true}
    }'
);

-- Test single JSON field GROUP BY with EXPLAIN
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT metadata->>'category' AS category, COUNT(*) AS count
FROM json_test_single
WHERE id @@@ paradedb.exists('metadata.category')
GROUP BY metadata->>'category'
ORDER BY category;

-- Execute the query
SELECT metadata->>'category' AS category, COUNT(*) AS count
FROM json_test_single
WHERE id @@@ paradedb.exists('metadata.category')
GROUP BY metadata->>'category'
ORDER BY category;

-- =========================================
-- Test 2: Multiple JSON field GROUP BY  
-- =========================================

-- Create test table for multiple fields
CREATE TABLE json_test_multiple (
    id SERIAL PRIMARY KEY,
    metadata JSONB
);

-- Insert test data
INSERT INTO json_test_multiple (metadata) VALUES
    ('{"category": "electronics", "brand": "Apple"}'),
    ('{"category": "electronics", "brand": "Samsung"}'),
    ('{"category": "electronics", "brand": "Apple"}'),
    ('{"category": "clothing", "brand": "Nike"}'),
    ('{"category": "clothing", "brand": "Nike"}');

-- Create BM25 index
CREATE INDEX idx_json_multiple ON json_test_multiple
USING bm25 (id, metadata)
WITH (
    key_field = 'id',
    json_fields = '{"metadata": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test multiple JSON field GROUP BY with EXPLAIN
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT metadata->>'category' AS category,
       metadata->>'brand' AS brand,
       COUNT(*) AS count
FROM json_test_multiple
WHERE id @@@ paradedb.exists('metadata.category') 
  AND id @@@ paradedb.exists('metadata.brand')
GROUP BY metadata->>'category', metadata->>'brand'
ORDER BY category, brand;

-- Execute the query
SELECT metadata->>'category' AS category,
       metadata->>'brand' AS brand,
       COUNT(*) AS count
FROM json_test_multiple
WHERE id @@@ paradedb.exists('metadata.category') 
  AND id @@@ paradedb.exists('metadata.brand')
GROUP BY metadata->>'category', metadata->>'brand'
ORDER BY category, brand;

-- =========================================
-- Test 3: JSON GROUP BY with COUNT aggregates
-- =========================================

-- Create test table for aggregates
CREATE TABLE json_test_aggregates (
    id SERIAL PRIMARY KEY,
    metadata JSONB
);

-- Insert test data
INSERT INTO json_test_aggregates (metadata) VALUES
    ('{"brand": "Apple", "price": 999}'),
    ('{"brand": "Samsung", "price": 799}'),
    ('{"brand": "Apple", "price": 1299}'),
    ('{"brand": "Nike", "price": 89}'),
    ('{"brand": "Nike", "price": 99}');

-- Create BM25 index
CREATE INDEX idx_json_aggregates ON json_test_aggregates
USING bm25 (id, metadata)
WITH (
    key_field = 'id',
    json_fields = '{"metadata": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test JSON field GROUP BY with COUNT with EXPLAIN
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT metadata->>'brand' AS brand, 
       COUNT(*) AS count
FROM json_test_aggregates
WHERE id @@@ paradedb.exists('metadata.brand')
GROUP BY metadata->>'brand'
ORDER BY brand;

-- Execute the query
SELECT metadata->>'brand' AS brand, 
       COUNT(*) AS count
FROM json_test_aggregates
WHERE id @@@ paradedb.exists('metadata.brand')
GROUP BY metadata->>'brand'
ORDER BY brand;

-- =========================================
-- Test 4: JSON GROUP BY with NULL handling
-- =========================================

-- Create test table for null handling
CREATE TABLE json_test_nulls (
    id SERIAL PRIMARY KEY,
    metadata JSONB
);

-- Insert test data with nulls and missing fields
INSERT INTO json_test_nulls (metadata) VALUES
    ('{"brand": "Apple", "category": "electronics"}'),
    ('{"brand": "Samsung"}'),     -- Missing category
    ('{}'),                       -- Empty JSON
    ('{"category": "clothing"}'); -- Missing brand

-- Create BM25 index
CREATE INDEX idx_json_nulls ON json_test_nulls
USING bm25 (id, metadata)
WITH (
    key_field = 'id',
    json_fields = '{"metadata": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test JSON GROUP BY with NULL handling
SELECT metadata->>'category' AS category, COUNT(*) AS count
FROM json_test_nulls
WHERE id @@@ paradedb.all()
GROUP BY metadata->>'category'
ORDER BY category NULLS FIRST;

-- =========================================
-- Test 5: Original example from issue
-- =========================================

-- Create ledger transactions table similar to original request
CREATE TABLE ledger_transactions (
    id SERIAL PRIMARY KEY,
    metadata_json JSONB,
    amount DECIMAL
);

-- Insert test data
INSERT INTO ledger_transactions (metadata_json, amount) VALUES
    ('{"reservation_id": "res_001", "user_id": "user_123"}', 100.00),
    ('{"reservation_id": "res_002", "user_id": "user_456"}', 250.00),
    ('{"reservation_id": "res_001", "user_id": "user_123"}', 75.00),
    ('{"reservation_id": "res_003", "user_id": "user_789"}', 180.00),
    ('{"reservation_id": "res_002", "user_id": "user_456"}', 95.00);

-- Create BM25 index
CREATE INDEX idx_ledger_json ON ledger_transactions
USING bm25 (id, metadata_json, amount)
WITH (
    key_field = 'id',
    json_fields = '{"metadata_json": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test the original example query with EXPLAIN
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT metadata_json->>'reservation_id' AS txn_key_value,
       COUNT(*) AS count
FROM ledger_transactions
WHERE id @@@ paradedb.exists('metadata_json.reservation_id')
GROUP BY metadata_json->>'reservation_id'
ORDER BY txn_key_value;

-- Execute the original example query
SELECT metadata_json->>'reservation_id' AS txn_key_value,
       COUNT(*) AS count
FROM ledger_transactions
WHERE id @@@ paradedb.exists('metadata_json.reservation_id')
GROUP BY metadata_json->>'reservation_id'
ORDER BY txn_key_value;

-- Clean up
DROP TABLE json_test_single;
DROP TABLE json_test_multiple;
DROP TABLE json_test_aggregates;
DROP TABLE json_test_nulls;
DROP TABLE ledger_transactions;
