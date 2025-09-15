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

-- GROUP BY ... ORDER BY ... LIMIT pushed down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT metadata->>'category' AS category, COUNT(*) AS count
FROM json_test_single
WHERE id @@@ paradedb.exists('metadata.category')
GROUP BY metadata->>'category'
ORDER BY 1
LIMIT 5;

SELECT metadata->>'category' AS category, COUNT(*) AS count
FROM json_test_single
WHERE id @@@ paradedb.exists('metadata.category')
GROUP BY metadata->>'category'
ORDER BY 1
LIMIT 5;

-- Ordering by count should not be pushed down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT metadata->>'category' AS category, COUNT(*) AS count
FROM json_test_single
WHERE id @@@ paradedb.exists('metadata.category')
GROUP BY metadata->>'category'
ORDER BY 2
LIMIT 5;

SELECT metadata->>'category' AS category, COUNT(*) AS count
FROM json_test_single
WHERE id @@@ paradedb.exists('metadata.category')
GROUP BY metadata->>'category'
ORDER BY 2
LIMIT 5;

DROP TABLE json_test_single CASCADE;
