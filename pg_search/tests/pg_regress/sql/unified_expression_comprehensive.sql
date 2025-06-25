-- Testing and Validation
-- Test all aspects of the unified expression evaluator

-- Create extension if not exists
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Drop table if it exists to ensure clean test
DROP TABLE IF EXISTS unified_test_products CASCADE;

-- Setup test table with diverse data types
CREATE TABLE unified_test_products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    category TEXT,
    price NUMERIC(10,2),
    in_stock BOOLEAN,
    tags TEXT[],
    created_at TIMESTAMP,
    metadata JSONB
);

-- Create search index using the correct syntax
CREATE INDEX unified_test_products_idx ON unified_test_products USING bm25 (
    id,
    name,
    description,
    price,
    in_stock
) WITH (key_field = 'id');

-- Insert comprehensive test data
INSERT INTO unified_test_products (name, description, category, price, in_stock, tags, created_at, metadata) VALUES
-- Tech products with search terms
('Apple iPhone 15', 'Latest smartphone with advanced camera', 'Electronics', 999.99, true, ARRAY['phone', 'apple', 'premium'], '2024-01-15', '{"brand": "Apple", "storage": "128GB"}'),
('Samsung Galaxy S24', 'Android smartphone with AI features', 'Electronics', 899.99, true, ARRAY['phone', 'samsung', 'android'], '2024-01-20', '{"brand": "Samsung", "storage": "256GB"}'),
('Apple MacBook Pro', 'Professional laptop for developers', 'Computers', 2499.99, false, ARRAY['laptop', 'apple', 'professional'], '2024-01-10', '{"brand": "Apple", "cpu": "M3"}'),

-- Home products without search terms
('Kitchen Blender', 'High-speed blender for smoothies', 'Kitchen', 149.99, true, ARRAY['blender', 'kitchen'], '2024-01-25', '{"power": "1000W"}'),
('Coffee Maker', 'Automatic drip coffee maker', 'Kitchen', 89.99, true, ARRAY['coffee', 'kitchen'], '2024-01-30', '{"capacity": "12cups"}'),
('Dining Table', 'Wooden dining table for 6 people', 'Furniture', 599.99, false, ARRAY['table', 'furniture'], '2024-02-01', '{"material": "oak"}'),

-- Mixed category products
('Apple Watch', 'Smartwatch with health tracking', 'Electronics', 399.99, true, ARRAY['watch', 'apple', 'health'], '2024-02-05', '{"brand": "Apple", "waterproof": true}'),
('Smart TV', 'Ultra HD smart television', 'Electronics', 799.99, true, ARRAY['tv', 'smart', 'entertainment'], '2024-02-10', '{"size": "55inch", "resolution": "4K"}'),
('Office Chair', 'Ergonomic office chair', 'Furniture', 299.99, true, ARRAY['chair', 'office', 'ergonomic'], '2024-02-15', '{"adjustable": true}'),

-- Products for edge cases
('Apple Juice', 'Fresh organic apple juice', 'Food', 4.99, true, ARRAY['juice', 'organic', 'apple'], '2024-02-20', '{"organic": true}'),
('Samsung Monitor', 'Professional monitor for design work', 'Electronics', 449.99, false, ARRAY['monitor', 'samsung', 'professional'], '2024-02-25', '{"brand": "Samsung", "size": "27inch"}');

SELECT name, description, category, price, in_stock, tags, created_at, metadata FROM unified_test_products;

-- Test 1: Basic Mixed Expression Validation
\echo '=== Test 1: Basic Mixed Expression Validation ==='

-- Test 1.1: OR with indexed and non-indexed predicates
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR category = 'Kitchen')
ORDER BY score DESC, id;

-- Test 1.2: AND with indexed and non-indexed predicates  
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' AND category = 'Electronics')
ORDER BY score DESC, id;

-- Test 1.3: NOT with mixed predicates
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE NOT (name @@@ 'Samsung' OR category = 'Furniture')
ORDER BY score DESC, id;

-- Test 2: Complex Nested Boolean Expressions
\echo '=== Test 2: Complex Nested Boolean Expressions ==='

-- Test 2.1: Nested AND/OR combinations
SELECT id, name, category, price, paradedb.score(id) as score
FROM unified_test_products 
WHERE ((name @@@ 'Apple' OR description @@@ 'smartphone') AND category = 'Electronics') 
   OR (category = 'Kitchen' AND price < 100)
ORDER BY score DESC, id;

-- Test 2.2: Complex NOT with nested expressions
-- NOTE: This test demonstrates a known limitation where PostgreSQL's query planner
-- incorrectly decomposes complex NOT expressions into multiple separate clauses.
-- The expression NOT ((name @@@ 'Apple' AND category = 'Electronics') OR (category = 'Furniture'))
-- gets parsed as multiple AND clauses instead of a single NOT expression.
-- This is a fundamental PostgreSQL query planning issue, not a unified evaluator issue.
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE NOT ((name @@@ 'Apple' AND category = 'Electronics') OR (category = 'Furniture'))
ORDER BY score DESC, id;

-- Test 2.3: Deep nesting with multiple operators
SELECT id, name, category, price, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR (description @@@ 'smartphone' AND price > 500)) 
  AND NOT (category = 'Food' OR in_stock = false)
ORDER BY score DESC, id;

-- Test 3: Data Type Comprehensive Testing
\echo '=== Test 3: Data Type Comprehensive Testing ==='

-- Test 3.1: Numeric field combinations
SELECT id, name, price, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR price BETWEEN 100 AND 500)
ORDER BY score DESC, id;

-- Test 3.2: Boolean field combinations
SELECT id, name, in_stock, paradedb.score(id) as score
FROM unified_test_products 
WHERE (description @@@ 'smartphone' OR in_stock = true)
ORDER BY score DESC, id;

-- Test 3.3: Array field combinations
SELECT id, name, tags, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR 'kitchen' = ANY(tags))
ORDER BY score DESC, id;

-- Test 3.4: Timestamp field combinations
SELECT id, name, created_at, paradedb.score(id) as score
FROM unified_test_products 
WHERE (description @@@ 'smartphone' OR created_at > '2024-02-01')
ORDER BY score DESC, id;

-- Test 3.5: JSONB field combinations
SELECT id, name, metadata, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR metadata->>'brand' = 'Samsung')
ORDER BY score DESC, id;

-- Test 4: Scoring Validation
\echo '=== Test 4: Scoring Validation ==='

-- Test 4.1: Verify BM25 scores are preserved for indexed predicates
SELECT id, name, paradedb.score(id) as score, 
       CASE WHEN paradedb.score(id) > 1.0 THEN 'BM25_SCORE' ELSE 'DEFAULT_SCORE' END as score_type
FROM unified_test_products 
WHERE name @@@ 'Apple'
ORDER BY score DESC, id;

-- Test 4.2: Verify default scores for non-indexed matches
SELECT id, name, category, paradedb.score(id) as score,
       CASE WHEN paradedb.score(id) = 1.0 THEN 'DEFAULT_SCORE' ELSE 'OTHER_SCORE' END as score_type
FROM unified_test_products 
WHERE category = 'Kitchen'
ORDER BY score DESC, id;

-- Test 4.3: Score combination in OR expressions
SELECT id, name, category, paradedb.score(id) as score,
       CASE 
         WHEN paradedb.score(id) > 1.0 THEN 'INDEXED_MATCH'
         WHEN paradedb.score(id) = 1.0 THEN 'NON_INDEXED_MATCH'
         ELSE 'NO_MATCH'
       END as match_type
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR category = 'Kitchen')
ORDER BY score DESC, id;

-- Test 4.4: Score combination in AND expressions
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' AND category = 'Electronics')
ORDER BY score DESC, id;

-- Test 5: Edge Cases and Error Conditions
\echo '=== Test 5: Edge Cases and Error Conditions ==='

-- Test 5.1: Empty search results with non-indexed fallback
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'NonExistentBrand' OR category = 'Kitchen')
ORDER BY score DESC, id;

-- Test 5.2: All conditions false
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'NonExistentBrand' AND category = 'NonExistentCategory')
ORDER BY score DESC, id;

-- Test 5.3: NULL handling in mixed expressions
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR category IS NULL)
ORDER BY score DESC, id;

-- Test 5.4: Complex expression with all OR conditions
SELECT id, name, category, price, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR category = 'Kitchen' OR price < 50 OR in_stock = false)
ORDER BY score DESC, id;

-- Test 6: Performance and Optimization Validation
\echo '=== Test 6: Performance and Optimization Validation ==='

-- Test 6.1: Large OR expression to test lazy evaluation
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR name @@@ 'Samsung' OR category = 'Electronics' 
       OR category = 'Kitchen' OR price > 1000 OR in_stock = true)
ORDER BY score DESC, id;

-- Test 6.2: Complex nested expression to test optimization
SELECT id, name, category, price, paradedb.score(id) as score
FROM unified_test_products 
WHERE ((name @@@ 'Apple' OR description @@@ 'smartphone') 
       AND (category = 'Electronics' OR category = 'Computers'))
   OR ((category = 'Kitchen' OR category = 'Furniture') 
       AND (price < 200 OR in_stock = true))
ORDER BY score DESC, id;

-- Test 6.3: Expression with repeated predicates to test caching
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR category = 'Electronics') 
  AND (name @@@ 'Apple' OR price > 100)
ORDER BY score DESC, id;

-- Test 7: Integration with PostgreSQL Features
\echo '=== Test 7: Integration with PostgreSQL Features ==='

-- Test 7.1: Mixed expressions with LIMIT and OFFSET
SELECT id, name, category, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR category = 'Electronics')
ORDER BY score DESC, id
LIMIT 5 OFFSET 1;

-- Test 7.2: Mixed expressions with GROUP BY
SELECT category, COUNT(*) as count, AVG(paradedb.score(id)) as avg_score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR category = 'Kitchen')
GROUP BY category
ORDER BY avg_score DESC;

-- Test 7.3: Mixed expressions in subquery
SELECT outer_query.id, outer_query.name, outer_query.score
FROM (
    SELECT id, name, category, paradedb.score(id) as score
    FROM unified_test_products 
    WHERE (name @@@ 'Apple' OR category = 'Electronics')
) outer_query
WHERE outer_query.score > 1.0
ORDER BY outer_query.score DESC;

-- Test 7.4: Mixed expressions with CTE
WITH high_score_products AS (
    SELECT id, name, category, paradedb.score(id) as score
    FROM unified_test_products 
    WHERE (name @@@ 'Apple' OR description @@@ 'smartphone')
)
SELECT * FROM high_score_products 
WHERE score > 1.0 OR category = 'Electronics'
ORDER BY score DESC;

-- Test 8: Backward Compatibility Validation
\echo '=== Test 8: Backward Compatibility Validation ==='

-- Test 8.1: Pure indexed queries (should work as before)
SELECT id, name, paradedb.score(id) as score
FROM unified_test_products 
WHERE name @@@ 'Apple'
ORDER BY score DESC, id;

-- Test 8.2: Pure non-indexed queries (should work as before)
SELECT id, name, category
FROM unified_test_products 
WHERE category = 'Kitchen'
ORDER BY id;

-- Test 8.3: Complex indexed-only boolean expressions
SELECT id, name, paradedb.score(id) as score
FROM unified_test_products 
WHERE (name @@@ 'Apple' OR description @@@ 'smartphone') AND NOT name @@@ 'Samsung'
ORDER BY score DESC, id;

-- Cleanup
DROP TABLE unified_test_products CASCADE; 
 