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

-- Test simple COUNT with JSON field filter
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
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

-- Test COUNT with specific category filter
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.category', 'electronics');

-- Execute the query
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.category', 'electronics');

-- =========================================
-- Test 3: COUNT with multiple JSON field filters
-- =========================================

-- Test COUNT with multiple JSON field conditions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
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
-- Test 4: SUM aggregate on JSON numeric fields (IS NOT SUPPORTED BY CUSTOM AGGREGATE SCAN YET)
-- =========================================

-- Test SUM on JSON numeric field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM((metadata->>'price')::numeric) as total_price
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.price');

-- Execute the query
SELECT SUM((metadata->>'price')::numeric) as total_price
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.price');

-- Test SUM with category filter
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM((metadata->>'price')::numeric) as electronics_total
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.category', 'electronics');

SELECT SUM((metadata->>'price')::numeric) as electronics_total
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.category', 'electronics');

-- Test SUM on data.stock field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM((data->>'stock')::integer) as total_stock
FROM json_agg_test 
WHERE id @@@ paradedb.exists('data.stock');

SELECT SUM((data->>'stock')::integer) as total_stock
FROM json_agg_test 
WHERE id @@@ paradedb.exists('data.stock');

-- =========================================
-- Test 5: AVG aggregate on JSON numeric fields (IS NOT SUPPORTED BY CUSTOM AGGREGATE SCAN YET)
-- =========================================

-- Test AVG on JSON numeric field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG((metadata->>'price')::numeric) as avg_price
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.price');

-- Execute the query
SELECT AVG((metadata->>'price')::numeric) as avg_price
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.price');

-- Test AVG with brand filter
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG((metadata->>'price')::numeric) as apple_avg_price
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.brand', 'Apple');

SELECT AVG((metadata->>'price')::numeric) as apple_avg_price
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.brand', 'Apple');

-- Test AVG on stock levels
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG((data->>'stock')::integer) as avg_stock
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.category', 'clothing');

SELECT AVG((data->>'stock')::integer) as avg_stock
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.category', 'clothing');

-- =========================================
-- Test 6: MIN/MAX aggregates on JSON fields (IS NOT SUPPORTED BY CUSTOM AGGREGATE SCAN YET)
-- =========================================

-- Test MIN/MAX on price
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    MIN((metadata->>'price')::numeric) as min_price,
    MAX((metadata->>'price')::numeric) as max_price
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.price');

-- Execute the query
SELECT 
    MIN((metadata->>'price')::numeric) as min_price,
    MAX((metadata->>'price')::numeric) as max_price
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.price');

-- Test MIN/MAX on specific category
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    MIN((metadata->>'price')::numeric) as min_electronics_price,
    MAX((metadata->>'price')::numeric) as max_electronics_price
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.category', 'electronics');

SELECT 
    MIN((metadata->>'price')::numeric) as min_electronics_price,
    MAX((metadata->>'price')::numeric) as max_electronics_price
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.category', 'electronics');

-- Test MIN/MAX on text fields (alphabetical)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    MIN(metadata->>'brand') as first_brand,
    MAX(metadata->>'brand') as last_brand
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.brand');

SELECT 
    MIN(metadata->>'brand') as first_brand,
    MAX(metadata->>'brand') as last_brand
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.brand');

-- =========================================
-- Test 7: Mixed aggregate functions in single query (IS NOT SUPPORTED BY CUSTOM AGGREGATE SCAN YET)
-- =========================================

-- Test multiple aggregates in one query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    COUNT(*) as item_count,
    SUM((metadata->>'price')::numeric) as total_value,
    AVG((metadata->>'price')::numeric) as avg_price,
    MIN((metadata->>'price')::numeric) as min_price,
    MAX((metadata->>'price')::numeric) as max_price,
    MIN(metadata->>'brand') as first_brand,
    MAX(metadata->>'brand') as last_brand
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.price');

-- Execute the query
SELECT 
    COUNT(*) as item_count,
    SUM((metadata->>'price')::numeric) as total_value,
    AVG((metadata->>'price')::numeric) as avg_price,
    MIN((metadata->>'price')::numeric) as min_price,
    MAX((metadata->>'price')::numeric) as max_price,
    MIN(metadata->>'brand') as first_brand,
    MAX(metadata->>'brand') as last_brand
FROM json_agg_test 
WHERE id @@@ paradedb.exists('metadata.price');

-- =========================================
-- Test 8: COUNT with boolean queries on JSON
-- =========================================

-- Test COUNT with boolean must query
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF, FORMAT JSON)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.exists('metadata.category'),
        paradedb.term('metadata.category', 'electronics')
    ]
);

-- Execute the query
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.exists('metadata.category'),
        paradedb.term('metadata.category', 'electronics')
    ]
);

-- =========================================
-- Test 9: Edge cases with JSON aggregates
-- =========================================

-- Test COUNT on empty result set
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.nonexistent', 'value');

SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.term('metadata.nonexistent', 'value');

-- Test COUNT with all() query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.all();

SELECT COUNT(*) 
FROM json_agg_test 
WHERE id @@@ paradedb.all();

-- =========================================
-- Test 10: Deep nested JSON aggregates
-- =========================================

-- Create table with deeply nested structures
CREATE TABLE json_deep_agg (
    id SERIAL PRIMARY KEY,
    nested_data JSONB
);

-- Insert deeply nested test data
INSERT INTO json_deep_agg (nested_data) VALUES
    ('{"level1": {"level2": {"level3": {"level4": {"value": "target1", "score": 100}}}}}'),
    ('{"level1": {"level2": {"level3": {"level4": {"value": "target2", "score": 85}}}}}'),
    ('{"level1": {"level2": {"level3": {"level4": {"value": "target1", "score": 95}}}}}'),
    ('{"level1": {"level2": {"alt_level3": {"level4": {"value": "target3", "score": 70}}}}}'),
    ('{"level1": {"different": {"level3": {"level4": {"value": "target1", "score": 80}}}}}');

-- Create BM25 index
CREATE INDEX idx_json_deep_agg ON json_deep_agg
USING bm25 (id, nested_data)
WITH (
    key_field = 'id',
    json_fields = '{"nested_data": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test COUNT on deeply nested path (4 levels deep)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_deep_agg 
WHERE id @@@ paradedb.exists('nested_data.level1.level2.level3.level4.value');


SELECT COUNT(*) 
FROM json_deep_agg 
WHERE id @@@ paradedb.exists('nested_data.level1.level2.level3.level4.value');

-- Test COUNT with deep filtering
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_deep_agg 
WHERE id @@@ paradedb.term('nested_data.level1.level2.level3.level4.value', 'target1');


SELECT COUNT(*) 
FROM json_deep_agg 
WHERE id @@@ paradedb.term('nested_data.level1.level2.level3.level4.value', 'target1');

-- =========================================
-- Test 11: Heterogeneous structures aggregates
-- =========================================

-- Create table with mixed document types
CREATE TABLE json_mixed_agg (
    id SERIAL PRIMARY KEY,
    doc JSONB
);

-- Insert various document structures
INSERT INTO json_mixed_agg (doc) VALUES
    -- IoT sensor data
    ('{"type": "sensor", "device": {"id": "temp001", "location": "warehouse", "readings": {"temperature": 22.5, "humidity": 65}}}'),
    ('{"type": "sensor", "device": {"id": "temp002", "location": "office", "readings": {"temperature": 21.0, "humidity": 55}}}'),
    
    -- User activity logs  
    ('{"type": "activity", "user": {"id": "user123", "session": {"duration": 1800, "pages": ["home", "products", "cart"]}}}'),
    ('{"type": "activity", "user": {"id": "user456", "session": {"duration": 3600, "pages": ["home", "about"]}}}'),
    
    -- Financial transactions
    ('{"type": "transaction", "payment": {"method": "credit", "amount": 99.99, "merchant": {"category": "retail", "name": "Store A"}}}'),
    ('{"type": "transaction", "payment": {"method": "debit", "amount": 25.50, "merchant": {"category": "food", "name": "Restaurant B"}}}');

-- Create BM25 index
CREATE INDEX idx_json_mixed_agg ON json_mixed_agg
USING bm25 (id, doc)
WITH (
    key_field = 'id',
    json_fields = '{"doc": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test COUNT by document type
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_mixed_agg 
WHERE id @@@ paradedb.term('doc.type', 'sensor');


SELECT COUNT(*) 
FROM json_mixed_agg 
WHERE id @@@ paradedb.term('doc.type', 'sensor');

-- Test COUNT on sensor data in specific location
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_mixed_agg 
WHERE id @@@ paradedb.term('doc.type', 'sensor')
  AND id @@@ paradedb.term('doc.device.location', 'warehouse');

SELECT COUNT(*) 
FROM json_mixed_agg 
WHERE id @@@ paradedb.term('doc.type', 'sensor')
  AND id @@@ paradedb.term('doc.device.location', 'warehouse');

-- Test COUNT on credit transactions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_mixed_agg 
WHERE id @@@ paradedb.term('doc.type', 'transaction')
  AND id @@@ paradedb.term('doc.payment.method', 'credit');

SELECT COUNT(*) 
FROM json_mixed_agg 
WHERE id @@@ paradedb.term('doc.type', 'transaction')
  AND id @@@ paradedb.term('doc.payment.method', 'credit');

-- =========================================
-- Test 12: Simple boolean queries on JSON
-- =========================================

-- Test COUNT with simple boolean must logic
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_mixed_agg 
WHERE id @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.term('doc.type', 'sensor'),
        paradedb.term('doc.device.location', 'office')
    ]
);

SELECT COUNT(*) 
FROM json_mixed_agg 
WHERE id @@@ paradedb.boolean(
    must := ARRAY[
        paradedb.term('doc.type', 'sensor'),
        paradedb.term('doc.device.location', 'office')
    ]
);

-- =========================================
-- Test 13: Array field aggregates
-- =========================================

-- Create table with array fields
CREATE TABLE json_array_agg (
    id SERIAL PRIMARY KEY,
    data JSONB
);

-- Insert data with arrays
INSERT INTO json_array_agg (data) VALUES
    ('{"tags": ["urgent", "customer", "billing"], "metadata": {"source": "email", "priority": "high"}}'),
    ('{"tags": ["feature", "enhancement"], "metadata": {"source": "github", "priority": "medium"}}'),
    ('{"tags": ["bug", "urgent", "frontend"], "metadata": {"source": "jira", "priority": "high"}}'),
    ('{"tags": ["documentation"], "metadata": {"source": "confluence", "priority": "low"}}');

-- Create BM25 index
CREATE INDEX idx_json_array_agg ON json_array_agg
USING bm25 (id, data)
WITH (
    key_field = 'id',
    json_fields = '{"data": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test COUNT on documents with specific tags
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_array_agg 
WHERE id @@@ paradedb.term('data.tags', 'urgent');

SELECT COUNT(*) 
FROM json_array_agg 
WHERE id @@@ paradedb.term('data.tags', 'urgent');

-- Test COUNT with high priority items
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_array_agg 
WHERE id @@@ paradedb.term('data.metadata.priority', 'high');


SELECT COUNT(*) 
FROM json_array_agg 
WHERE id @@@ paradedb.term('data.metadata.priority', 'high');

-- =========================================
-- Test 14: Special characters and edge cases
-- =========================================

-- Create table with special JSON keys
CREATE TABLE json_special_agg (
    id SERIAL PRIMARY KEY,
    payload JSONB
);

-- Insert data with special characters
INSERT INTO json_special_agg (payload) VALUES
    ('{"user-profile": {"first_name": "John", "email@work": "john@company.com", "settings.theme": "dark"}}'),
    ('{"user-profile": {"first_name": "Jane", "email@work": "jane@company.com", "settings.theme": "light"}}'),
    ('{"api-response": {"status_code": 200, "response.time": 150, "cache-hit": true}}'),
    ('{"api-response": {"status_code": 404, "response.time": 50, "cache-hit": false}}');

-- Create BM25 index
CREATE INDEX idx_json_special_agg ON json_special_agg
USING bm25 (id, payload)
WITH (
    key_field = 'id',
    json_fields = '{"payload": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test COUNT with special character fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_special_agg 
WHERE id @@@ paradedb.exists('payload.user-profile.email@work');

SELECT COUNT(*) 
FROM json_special_agg 
WHERE id @@@ paradedb.exists('payload.user-profile.email@work');

-- Test COUNT on API responses
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) 
FROM json_special_agg 
WHERE id @@@ paradedb.term('payload.api-response.status_code', '200');

SELECT COUNT(*) 
FROM json_special_agg 
WHERE id @@@ paradedb.term('payload.api-response.status_code', '200');

-- Clean up
DROP TABLE json_agg_test;
DROP TABLE json_deep_agg;
DROP TABLE json_mixed_agg;
DROP TABLE json_array_agg;
DROP TABLE json_special_agg;
