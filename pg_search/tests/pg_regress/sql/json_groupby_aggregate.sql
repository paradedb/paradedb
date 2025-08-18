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
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
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
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
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
-- Test 3: JSON GROUP BY with various aggregate functions (IS NOT SUPPORTED BY CUSTOM AGGREGATE SCAN YET)
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
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
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

-- Test JSON field GROUP BY with SUM with EXPLAIN
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT metadata->>'category' AS category, 
       SUM((metadata->>'price')::numeric) AS total_price
FROM json_test_aggregates
WHERE id @@@ paradedb.exists('metadata.price')
GROUP BY metadata->>'category'
ORDER BY category;

-- Execute the query
SELECT metadata->>'category' AS category, 
       SUM((metadata->>'price')::numeric) AS total_price
FROM json_test_aggregates
WHERE id @@@ paradedb.exists('metadata.price')
GROUP BY metadata->>'category'
ORDER BY category;

-- Test JSON field GROUP BY with AVG with EXPLAIN
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT metadata->>'brand' AS brand, 
       AVG((metadata->>'price')::numeric) AS avg_price,
       COUNT(*) AS item_count
FROM json_test_aggregates
WHERE id @@@ paradedb.exists('metadata.price')
GROUP BY metadata->>'brand'
ORDER BY brand;

-- Execute the query
SELECT metadata->>'brand' AS brand, 
       AVG((metadata->>'price')::numeric) AS avg_price,
       COUNT(*) AS item_count
FROM json_test_aggregates
WHERE id @@@ paradedb.exists('metadata.price')
GROUP BY metadata->>'brand'
ORDER BY brand;

-- Test JSON field GROUP BY with MIN/MAX with EXPLAIN
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT metadata->>'category' AS category, 
       MIN((metadata->>'price')::numeric) AS min_price,
       MAX((metadata->>'price')::numeric) AS max_price,
       COUNT(*) AS item_count
FROM json_test_aggregates
WHERE id @@@ paradedb.exists('metadata.price')
GROUP BY metadata->>'category'
ORDER BY category;

-- Execute the query
SELECT metadata->>'category' AS category, 
       MIN((metadata->>'price')::numeric) AS min_price,
       MAX((metadata->>'price')::numeric) AS max_price,
       COUNT(*) AS item_count
FROM json_test_aggregates
WHERE id @@@ paradedb.exists('metadata.price')
GROUP BY metadata->>'category'
ORDER BY category;

-- Test JSON field GROUP BY with multiple aggregates with EXPLAIN
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT metadata->>'brand' AS brand, 
       COUNT(*) AS item_count,
       SUM((metadata->>'price')::numeric) AS total_value,
       AVG((metadata->>'price')::numeric) AS avg_price,
       MIN((metadata->>'price')::numeric) AS min_price,
       MAX((metadata->>'price')::numeric) AS max_price
FROM json_test_aggregates
WHERE id @@@ paradedb.exists('metadata.price')
GROUP BY metadata->>'brand'
ORDER BY brand;

-- Execute the query
SELECT metadata->>'brand' AS brand, 
       COUNT(*) AS item_count,
       SUM((metadata->>'price')::numeric) AS total_value,
       AVG((metadata->>'price')::numeric) AS avg_price,
       MIN((metadata->>'price')::numeric) AS min_price,
       MAX((metadata->>'price')::numeric) AS max_price
FROM json_test_aggregates
WHERE id @@@ paradedb.exists('metadata.price')
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
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT metadata->>'category' AS category, COUNT(*) AS count
FROM json_test_nulls
WHERE id @@@ paradedb.all()
GROUP BY metadata->>'category'
ORDER BY category NULLS FIRST;

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
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
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

-- =========================================
-- Test 6: Deep nested JSON fields
-- =========================================

-- Create test table with deeply nested JSON
CREATE TABLE json_test_deep (
    id SERIAL PRIMARY KEY,
    config JSONB
);

-- Insert test data with varying nesting levels
INSERT INTO json_test_deep (config) VALUES
    ('{"user": {"profile": {"settings": {"theme": "dark", "region": "us-east"}}}}'),
    ('{"user": {"profile": {"settings": {"theme": "light", "region": "us-west"}}}}'),
    ('{"user": {"profile": {"settings": {"theme": "dark", "region": "eu-central"}}}}'),
    ('{"user": {"profile": {"settings": {"theme": "auto", "region": "us-east"}}}}'),
    ('{"user": {"profile": {"settings": {"theme": "light", "region": "us-east"}}}}');

-- Create BM25 index with nested JSON fields
CREATE INDEX idx_json_deep ON json_test_deep
USING bm25 (id, config)
WITH (
    key_field = 'id',
    json_fields = '{"config": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test GROUP BY on deeply nested field (3 levels deep)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT config->'user'->'profile'->'settings'->>'theme' AS theme,
       config->'user'->'profile'->'settings'->>'region' AS region,
       COUNT(*) AS count
FROM json_test_deep
WHERE id @@@ paradedb.exists('config.user.profile.settings.theme')
  AND id @@@ paradedb.exists('config.user.profile.settings.region')
GROUP BY config->'user'->'profile'->'settings'->>'theme',
         config->'user'->'profile'->'settings'->>'region'
ORDER BY theme, region;

-- Execute the query
SELECT config->'user'->'profile'->'settings'->>'theme' AS theme,
       config->'user'->'profile'->'settings'->>'region' AS region,
       COUNT(*) AS count
FROM json_test_deep
WHERE id @@@ paradedb.exists('config.user.profile.settings.theme')
  AND id @@@ paradedb.exists('config.user.profile.settings.region')
GROUP BY config->'user'->'profile'->'settings'->>'theme',
         config->'user'->'profile'->'settings'->>'region'
ORDER BY theme, region;

-- =========================================
-- Test 7: Heterogeneous JSON structures
-- =========================================

-- Create test table with varying JSON structures
CREATE TABLE json_test_mixed (
    id SERIAL PRIMARY KEY,
    data JSONB
);

-- Insert data with completely different JSON structures
INSERT INTO json_test_mixed (data) VALUES
    -- E-commerce products
    ('{"type": "product", "category": "electronics", "brand": "Apple", "price": 999, "specs": {"cpu": "M1", "ram": "8GB"}}'),
    ('{"type": "product", "category": "electronics", "brand": "Samsung", "price": 799, "specs": {"screen": "OLED", "storage": "256GB"}}'),
    ('{"type": "product", "category": "clothing", "brand": "Nike", "price": 89, "details": {"size": "L", "color": "blue"}}'),
    
    -- User profiles  
    ('{"type": "user", "profile": {"name": "John", "location": {"country": "USA", "city": "NYC"}}, "preferences": {"theme": "dark"}}'),
    ('{"type": "user", "profile": {"name": "Jane", "location": {"country": "USA", "city": "LA"}}, "preferences": {"theme": "light"}}'),
    ('{"type": "user", "profile": {"name": "Bob", "location": {"country": "Canada", "city": "Toronto"}}, "preferences": {"theme": "dark"}}'),
    
    -- Event logs
    ('{"type": "event", "event": {"name": "login", "timestamp": "2024-01-01", "source": {"app": "web", "version": "1.0"}}}'),
    ('{"type": "event", "event": {"name": "logout", "timestamp": "2024-01-01", "source": {"app": "mobile", "version": "2.0"}}}'),
    ('{"type": "event", "event": {"name": "login", "timestamp": "2024-01-02", "source": {"app": "web", "version": "1.1"}}}');

-- Create BM25 index 
CREATE INDEX idx_json_mixed ON json_test_mixed
USING bm25 (id, data)
WITH (
    key_field = 'id',
    json_fields = '{"data": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test GROUP BY on heterogeneous structures - group by type
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT data->>'type' AS object_type, COUNT(*) AS count
FROM json_test_mixed
WHERE id @@@ paradedb.exists('data.type')
GROUP BY data->>'type'
ORDER BY object_type;

-- Execute the query
SELECT data->>'type' AS object_type, COUNT(*) AS count
FROM json_test_mixed
WHERE id @@@ paradedb.exists('data.type')
GROUP BY data->>'type'
ORDER BY object_type;

-- Test GROUP BY on products only (filtering by type)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT data->>'category' AS category, data->>'brand' AS brand, COUNT(*) AS count
FROM json_test_mixed
WHERE id @@@ paradedb.term('data.type', 'product')
  AND id @@@ paradedb.exists('data.category')
  AND id @@@ paradedb.exists('data.brand')
GROUP BY data->>'category', data->>'brand'
ORDER BY category, brand;

-- Execute the query
SELECT data->>'category' AS category, data->>'brand' AS brand, COUNT(*) AS count
FROM json_test_mixed
WHERE id @@@ paradedb.term('data.type', 'product')
  AND id @@@ paradedb.exists('data.category')
  AND id @@@ paradedb.exists('data.brand')
GROUP BY data->>'category', data->>'brand'
ORDER BY category, brand;

-- Test GROUP BY on user locations (nested field access)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT data->'profile'->'location'->>'country' AS country, 
       COUNT(*) AS user_count
FROM json_test_mixed
WHERE id @@@ paradedb.term('data.type', 'user')
  AND id @@@ paradedb.exists('data.profile.location.country')
GROUP BY data->'profile'->'location'->>'country'
ORDER BY country;

-- Execute the query
SELECT data->'profile'->'location'->>'country' AS country, 
       COUNT(*) AS user_count
FROM json_test_mixed
WHERE id @@@ paradedb.term('data.type', 'user')
  AND id @@@ paradedb.exists('data.profile.location.country')
GROUP BY data->'profile'->'location'->>'country'
ORDER BY country;

-- =========================================
-- Test 8: Mixed JSON operators (-> vs ->>)
-- =========================================

-- Create test table for operator mixing
CREATE TABLE json_test_operators (
    id SERIAL PRIMARY KEY,
    payload JSONB
);

-- Insert test data
INSERT INTO json_test_operators (payload) VALUES
    ('{"metadata": {"tags": ["urgent", "customer"], "priority": "high", "assignee": {"name": "Alice", "team": "support"}}, "team": "support"}'),
    ('{"metadata": {"tags": ["feature", "backend"], "priority": "medium", "assignee": {"name": "Bob", "team": "engineering"}}, "team": "engineering"}'),
    ('{"metadata": {"tags": ["bug", "frontend"], "priority": "high", "assignee": {"name": "Alice", "team": "engineering"}}, "team": "engineering"}'),
    ('{"metadata": {"tags": ["urgent", "billing"], "priority": "low", "assignee": {"name": "Carol", "team": "support"}}, "team": "support"}'),
    ('{"metadata": {"tags": ["feature", "api"], "priority": "medium", "assignee": {"name": "Bob", "team": "engineering"}}, "team": "engineering"}');

-- Create BM25 index
CREATE INDEX idx_json_operators ON json_test_operators
USING bm25 (id, payload)
WITH (
    key_field = 'id',
    json_fields = '{"payload": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test mixing -> and ->> operators in GROUP BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT payload->'metadata'->>'priority' AS priority_text,
       COUNT(*) AS count
FROM json_test_operators
WHERE id @@@ paradedb.exists('payload.metadata.priority')
GROUP BY payload->'metadata'->>'priority'
ORDER BY priority_text;

-- Execute the query
SELECT payload->'metadata'->>'priority' AS priority_text,
       COUNT(*) AS count
FROM json_test_operators
WHERE id @@@ paradedb.exists('payload.metadata.priority')
GROUP BY payload->'metadata'->>'priority'
ORDER BY priority_text;

-- Test with simpler assignee team field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT payload->>'team' AS team,
       COUNT(*) AS count
FROM json_test_operators
WHERE id @@@ paradedb.exists('payload.team')
GROUP BY payload->>'team'
ORDER BY team;

-- Execute the query
SELECT payload->>'team' AS team,
       COUNT(*) AS count
FROM json_test_operators
WHERE id @@@ paradedb.exists('payload.team')
GROUP BY payload->>'team'
ORDER BY team;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT payload->'metadata'->>'priority' AS priority_text,
       payload->'metadata'->'assignee'->>'team' AS team,
       COUNT(*) AS count
FROM json_test_operators
WHERE id @@@ paradedb.exists('payload.metadata.priority')
  AND id @@@ paradedb.exists('payload.metadata.assignee.team')
GROUP BY payload->'metadata'->>'priority', payload->'metadata'->'assignee'->>'team'
ORDER BY priority_text, team;

-- Execute the query
SELECT payload->'metadata'->>'priority' AS priority_text,
       payload->'metadata'->'assignee'->>'team' AS team,
       COUNT(*) AS count
FROM json_test_operators
WHERE id @@@ paradedb.exists('payload.metadata.priority')
  AND id @@@ paradedb.exists('payload.metadata.assignee.team')
GROUP BY payload->'metadata'->>'priority', payload->'metadata'->'assignee'->>'team'
ORDER BY priority_text, team;

-- =========================================
-- Test 9: Array elements and complex nesting
-- =========================================

-- Create test table with arrays and complex structures
CREATE TABLE json_test_complex (
    id SERIAL PRIMARY KEY,
    document JSONB
);

-- Insert complex nested data with arrays
INSERT INTO json_test_complex (document) VALUES
    ('{"source": {"system": "crm", "version": "2.1"}, "tags": ["customer", "vip"], "metrics": {"score": 85, "category": "A"}}'),
    ('{"source": {"system": "crm", "version": "2.0"}, "tags": ["prospect"], "metrics": {"score": 70, "category": "B"}}'),
    ('{"source": {"system": "billing", "version": "1.5"}, "tags": ["customer", "enterprise"], "metrics": {"score": 95, "category": "A"}}'),
    ('{"source": {"system": "support", "version": "3.0"}, "tags": ["internal"], "metrics": {"score": 60, "category": "C"}}'),
    ('{"source": {"system": "crm", "version": "2.1"}, "tags": ["customer"], "metrics": {"score": 80, "category": "B"}}');

-- Create BM25 index
CREATE INDEX idx_json_complex ON json_test_complex
USING bm25 (id, document)
WITH (
    key_field = 'id',
    json_fields = '{"document": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test GROUP BY on nested system and category
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT document->'source'->>'system' AS source_system,
       document->'metrics'->>'category' AS metric_category,
       COUNT(*) AS count,
       AVG((document->'metrics'->>'score')::numeric) AS avg_score
FROM json_test_complex
WHERE id @@@ paradedb.exists('document.source.system')
  AND id @@@ paradedb.exists('document.metrics.category')
GROUP BY document->'source'->>'system', document->'metrics'->>'category'
ORDER BY source_system, metric_category;

-- Execute the query
SELECT document->'source'->>'system' AS source_system,
       document->'metrics'->>'category' AS metric_category,
       COUNT(*) AS count,
       AVG((document->'metrics'->>'score')::numeric) AS avg_score
FROM json_test_complex
WHERE id @@@ paradedb.exists('document.source.system')
  AND id @@@ paradedb.exists('document.metrics.category')
GROUP BY document->'source'->>'system', document->'metrics'->>'category'
ORDER BY source_system, metric_category;

-- Test GROUP BY with comprehensive aggregates on scores (IS NOT SUPPORTED BY CUSTOM AGGREGATE SCAN YET)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT document->'metrics'->>'category' AS metric_category,
       COUNT(*) AS total_records,
       SUM((document->'metrics'->>'score')::numeric) AS total_score,
       AVG((document->'metrics'->>'score')::numeric) AS avg_score,
       MIN((document->'metrics'->>'score')::numeric) AS min_score,
       MAX((document->'metrics'->>'score')::numeric) AS max_score
FROM json_test_complex
WHERE id @@@ paradedb.exists('document.metrics.score')
GROUP BY document->'metrics'->>'category'
ORDER BY metric_category;

-- Execute the query
SELECT document->'metrics'->>'category' AS metric_category,
       COUNT(*) AS total_records,
       SUM((document->'metrics'->>'score')::numeric) AS total_score,
       AVG((document->'metrics'->>'score')::numeric) AS avg_score,
       MIN((document->'metrics'->>'score')::numeric) AS min_score,
       MAX((document->'metrics'->>'score')::numeric) AS max_score
FROM json_test_complex
WHERE id @@@ paradedb.exists('document.metrics.score')
GROUP BY document->'metrics'->>'category'
ORDER BY metric_category;

-- =========================================
-- Test 10: Edge cases with special characters
-- =========================================

-- Create test table with special JSON keys
CREATE TABLE json_test_special (
    id SERIAL PRIMARY KEY,
    content JSONB
);

-- Insert data with special characters in keys
INSERT INTO json_test_special (content) VALUES
    ('{"user-info": {"first_name": "John", "last-name": "Doe", "email@domain": "work"}}'),
    ('{"user-info": {"first_name": "Jane", "last-name": "Smith", "email@domain": "personal"}}'),
    ('{"user-info": {"first_name": "Bob", "last-name": "Jones", "email@domain": "work"}}'),
    ('{"user-info": {"first_name": "Alice", "last-name": "Brown", "email@domain": "work"}}');

-- Create BM25 index
CREATE INDEX idx_json_special ON json_test_special
USING bm25 (id, content)
WITH (
    key_field = 'id',
    json_fields = '{"content": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test GROUP BY with special characters in JSON keys
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT content->'user-info'->>'email@domain' AS email_type,
       COUNT(*) AS count
FROM json_test_special
WHERE id @@@ paradedb.exists('content.user-info.email@domain')
GROUP BY content->'user-info'->>'email@domain'
ORDER BY email_type;

-- Execute the query
SELECT content->'user-info'->>'email@domain' AS email_type,
       COUNT(*) AS count
FROM json_test_special
WHERE id @@@ paradedb.exists('content.user-info.email@domain')
GROUP BY content->'user-info'->>'email@domain'
ORDER BY email_type;

-- Clean up
DROP TABLE json_test_single;
DROP TABLE json_test_multiple;
DROP TABLE json_test_aggregates;
DROP TABLE json_test_nulls;
DROP TABLE ledger_transactions;
DROP TABLE json_test_deep;
DROP TABLE json_test_mixed;
DROP TABLE json_test_operators;
DROP TABLE json_test_complex;
DROP TABLE json_test_special;
