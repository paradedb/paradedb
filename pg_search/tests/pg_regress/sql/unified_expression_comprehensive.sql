-- Test case for demonstrating the issue where BM25 scores return null 
-- when not all predicates are indexed in the BM25 index
-- This is a simpler reproduction case than join scenarios

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;
SET paradedb.enable_mixed_fast_field_exec = true;

-- Setup test table
DROP TABLE IF EXISTS products;

CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    price DECIMAL(10,2),
    category_id INTEGER,
    category_name TEXT,
    in_stock BOOLEAN,
    created_at TIMESTAMP DEFAULT '2025-01-01 12:00:00'::timestamp,
    rating REAL,
    tags TEXT[]
);

-- Insert test data
INSERT INTO products (name, description, price, category_id, category_name, in_stock, rating, tags) VALUES
('Apple iPhone 14', 'Latest Apple smartphone with great camera', 999.99, 1, 'Casual', true, 4.5, ARRAY['smartphone', 'apple']),
('MacBook Pro', 'Powerful Apple laptop for professionals', 2499.99, 1, 'Electronics', true, 4.8, ARRAY['laptop', 'apple']),
-- ('Apple iPhone 13', 'Latest Apple smartphone with medium camera', 899.99, 1, 'Casual', true, 4.5, ARRAY['smartphone', 'apple']),
('Nike Air Max', 'Comfortable running shoes for athletes', 149.99, 2, 'Footwear', true, 4.2, ARRAY['shoes', 'running']),
('Samsung Galaxy', 'Android smartphone with excellent display', 899.99, 1, 'Electronics', false, 4.3, ARRAY['smartphone', 'android']),
('Adidas Ultraboost', 'Premium running shoes with boost technology', 179.99, 2, 'Footwear', true, 4.6, ARRAY['shoes', 'running', 'premium']),
('Nike Normal', 'Comfortable running shoes for athletes and technology enthusiasts', 149.99, 2, 'Footwear', false, 3.9, ARRAY['shoes', 'casual']),
('Apple Watch', 'Smartwatch with health tracking features', 399.99, 1, 'Electronics', true, 4.4, ARRAY['watch', 'apple']),
('Sony Headphones', 'Noise-canceling headphones for music lovers', 299.99, 1, 'Electronics', true, 4.7, ARRAY['headphones', 'audio']),
('Running Socks', 'Moisture-wicking socks for athletes', 19.99, 2, 'Footwear', true, 4.0, ARRAY['socks', 'running']),
('Budget Phone', 'Affordable smartphone for basic needs', 199.99, 1, 'Electronics', false, 3.5, NULL),
('Budget Tablet', 'Affordable tablet for basic needs', 199.99, 1, 'Garbage', false, 3.5, NULL);


-- Create BM25 index that only includes some columns (name, description)
-- Note: price, category_id, category_name, in_stock, rating, tags are NOT in the BM25 index
CREATE INDEX products_bm25_idx ON products USING bm25 (
    id,
    name,
    description
) WITH (key_field = 'id');

-- Test Case 1: Query using only indexed columns - should return proper scores
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    paradedb.score(id) as score
FROM products 
WHERE name @@@ 'Apple' OR description @@@ 'smartphone'
ORDER BY score DESC;

SELECT 
    id,
    name,
    paradedb.score(id) as score
FROM products 
WHERE name @@@ 'Apple' OR description @@@ 'smartphone'
ORDER BY score DESC;

-- Test Case 2: Query using indexed + non-indexed columns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'Apple' OR description @@@ 'smartphone') 
  AND category_name = 'Electronics'
ORDER BY score DESC;

SELECT 
    id,
    name,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'Apple' OR description @@@ 'smartphone') 
  AND category_name = 'Electronics'
ORDER BY score DESC;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'Apple' OR description @@@ 'smartphone') 
  OR category_name = 'Electronics'
ORDER BY score DESC;

SELECT 
    id,
    name,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'Apple' OR description @@@ 'smartphone') 
  OR category_name = 'Electronics'
ORDER BY score DESC;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'Apple' AND description @@@ 'smartphone') 
  OR category_name = 'Electronics'
ORDER BY score DESC;

SELECT 
    id,
    name,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'Apple' AND description @@@ 'smartphone') 
  OR category_name = 'Electronics'
ORDER BY score DESC;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'Apple' AND description @@@ 'smartphone') 
  OR TRUE OR category_name = 'Electronics'
ORDER BY score DESC;

SELECT 
    id,
    name,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'Apple' AND description @@@ 'smartphone') 
  OR TRUE OR category_name = 'Electronics'
ORDER BY score DESC;

-- Test Case 3: Another example with price filter (non-indexed)
-- Should show the same issue - scores become null due to non-indexed predicate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    price,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'running' 
  AND price < 200.00
ORDER BY score DESC;

SELECT 
    id,
    name,
    price,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'running' 
  AND price < 200.00
ORDER BY score DESC;

-- Test Case 4: Mixed predicates with boolean non-indexed column
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    in_stock,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'technology' 
  AND in_stock = true
ORDER BY score DESC;

SELECT 
    id,
    name,
    in_stock,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'technology' 
  AND in_stock = true
ORDER BY score DESC;

-- For comparison: Show that when all predicates are on indexed columns, scores work
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    description,
    paradedb.score(id) as score
FROM products 
WHERE name @@@ 'MacBook' 
  AND description @@@ 'laptop'
ORDER BY score DESC;

SELECT 
    id,
    name,
    description,
    paradedb.score(id) as score
FROM products 
WHERE name @@@ 'MacBook' 
  AND description @@@ 'laptop'
ORDER BY score DESC;

-- Test Case 5: Complex query with multiple non-indexed predicates
-- This should clearly show scores being null even when some predicates could contribute
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    category_name,
    price,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'shoes' OR description @@@ 'running')
  AND category_name = 'Footwear'
  AND price BETWEEN 100.00 AND 200.00
ORDER BY score DESC;

SELECT 
    id,
    name,
    category_name,
    price,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'shoes' OR description @@@ 'running')
  AND category_name = 'Footwear'
  AND price BETWEEN 100.00 AND 200.00
ORDER BY score DESC; 

-- Test Case 6: Multiple AND conditions with different data types
-- Tests heap filtering with integer, decimal, and text non-indexed predicates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    category_id,
    price,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'Apple'
  AND category_id = 1
  AND price > 500.00
  AND category_name = 'Electronics'
ORDER BY score DESC;

SELECT 
    id,
    name,
    category_id,
    price,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'Apple'
  AND category_id = 1
  AND price > 500.00
  AND category_name = 'Electronics'
ORDER BY score DESC;

-- Test Case 7: Complex nested OR/AND combinations
-- Tests recursive clause extraction and combination
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    price,
    in_stock,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'smartphone'
  AND (
    (price < 500.00 AND in_stock = true) OR 
    (price > 800.00 AND category_name = 'Electronics')
  )
ORDER BY score DESC;

SELECT 
    id,
    name,
    price,
    in_stock,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'smartphone'
  AND (
    (price < 500.00 AND in_stock = true) OR 
    (price > 800.00 AND category_name = 'Electronics')
  )
ORDER BY score DESC;

-- Test Case 8: Real number (REAL) filtering
-- Tests heap filtering with floating-point comparisons
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'running'
  AND rating >= 4.0
ORDER BY score DESC;

SELECT 
    id,
    name,
    rating,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'running'
  AND rating >= 4.0
ORDER BY score DESC;

-- Test Case 9: NULL value handling
-- Tests heap filtering with NULL checks
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    tags,
    paradedb.score(id) as score
FROM products 
WHERE name @@@ 'phone'
  AND tags IS NULL
ORDER BY score DESC;

SELECT 
    id,
    name,
    tags,
    paradedb.score(id) as score
FROM products 
WHERE name @@@ 'phone'
  AND tags IS NULL
ORDER BY score DESC;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    tags,
    paradedb.score(id) as score
FROM products 
WHERE name @@@ 'phone'
  OR tags IS NULL
ORDER BY score DESC;

SELECT 
    id,
    name,
    tags,
    paradedb.score(id) as score
FROM products 
WHERE name @@@ 'phone'
  OR tags IS NULL
ORDER BY score DESC;

-- Test Case 10: NOT NULL filtering
-- Tests heap filtering with NOT NULL predicates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    tags,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'Apple'
  AND tags IS NOT NULL
ORDER BY score DESC;

SELECT 
    id,
    name,
    tags,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'Apple'
  AND tags IS NOT NULL
ORDER BY score DESC;

-- Test Case 11: Multiple OR conditions with non-indexed predicates
-- Tests complex OR logic in heap filtering
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    price,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'technology'
  AND (
    price < 100.00 OR 
    category_name = 'Electronics' OR
    in_stock = false
  )
ORDER BY score DESC;

SELECT 
    id,
    name,
    price,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'technology'
  AND (
    price < 100.00 OR 
    category_name = 'Electronics' OR
    in_stock = false
  )
ORDER BY score DESC;

-- Test Case 12: Edge case - all tuples filtered out
-- Tests behavior when heap filtering eliminates all results
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    price,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'running'
  AND price > 1000.00  -- Should filter out all running items
ORDER BY score DESC;

SELECT 
    id,
    name,
    price,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'running'
  AND price > 1000.00  -- Should filter out all running items
ORDER BY score DESC;

-- Test Case 13: Edge case - no search predicates, only non-indexed
-- Tests heap filtering when there are no indexed predicates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    price,
    in_stock,
    paradedb.score(id) as score
FROM products 
WHERE price BETWEEN 100.00 AND 300.00
  AND in_stock = true
ORDER BY score DESC;

SELECT 
    id,
    name,
    price,
    in_stock,
    paradedb.score(id) as score
FROM products 
WHERE price BETWEEN 100.00 AND 300.00
  AND in_stock = true
ORDER BY score DESC;

-- Test Case 14: Array operations (if supported)
-- Tests heap filtering with array predicates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    tags,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'Apple'
  AND 'apple' = ANY(tags)
ORDER BY score DESC;

SELECT 
    id,
    name,
    tags,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'Apple'
  AND 'apple' = ANY(tags)
ORDER BY score DESC;

-- Test Case 15: Timestamp filtering
-- Tests heap filtering with timestamp predicates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    created_at,
    paradedb.score(id) as score
FROM products 
WHERE name @@@ 'Apple'
  AND created_at > '2024-01-01 00:00:00'::timestamp
ORDER BY score DESC;

SELECT 
    id,
    name,
    created_at,
    paradedb.score(id) as score
FROM products 
WHERE name @@@ 'Apple'
  AND created_at > '2024-01-01 00:00:00'::timestamp
ORDER BY score DESC;

-- Test Case 16: Combined numeric comparisons
-- Tests multiple numeric predicate combinations
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    price,
    rating,
    category_id,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'shoes'
  AND price <= 200.00
  AND rating > 4.0
  AND category_id = 2
ORDER BY score DESC;

SELECT 
    id,
    name,
    price,
    rating,
    category_id,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'shoes'
  AND price <= 200.00
  AND rating > 4.0
  AND category_id = 2
ORDER BY score DESC;

-- Test Case 17: String pattern matching
-- Tests heap filtering with LIKE predicates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'technology'
  AND category_name LIKE 'Elect%'
ORDER BY score DESC;

SELECT 
    id,
    name,
    category_name,
    paradedb.score(id) as score
FROM products 
WHERE description @@@ 'technology'
  AND category_name LIKE 'Elect%'
ORDER BY score DESC;

-- Test Case 18: Mixed boolean logic complexity
-- Tests deeply nested boolean expressions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    id,
    name,
    price,
    in_stock,
    rating,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'Apple' OR description @@@ 'smartphone')
  AND (
    (price > 500.00 AND in_stock = true) OR
    (price < 300.00 AND rating >= 4.0) OR
    (category_name = 'Electronics' AND rating > 4.5)
  )
ORDER BY score DESC;

SELECT 
    id,
    name,
    price,
    in_stock,
    rating,
    paradedb.score(id) as score
FROM products 
WHERE (name @@@ 'Apple' OR description @@@ 'smartphone')
  AND (
    (price > 500.00 AND in_stock = true) OR
    (price < 300.00 AND rating >= 4.0) OR
    (category_name = 'Electronics' AND rating > 4.5)
  )
ORDER BY score DESC; 

SET paradedb.enable_filter_pushdown = true;

-- Test Case 19.1: Test that subqueries on the RHS of a heap filter which don't match anything
-- result in an error.
SELECT
  products.id
FROM products
WHERE
  (products.id @@@ paradedb.all())
  AND (products.name ILIKE ANY (array['%Socks%']))
  AND (products.created_at < (SELECT created_at FROM products WHERE products.id = 1978) OR products.id < 1978 AND products.created_at = (SELECT created_at FROM products WHERE products.id = 1978))
ORDER BY products.created_at DESC, products.id DESC
LIMIT 100;

-- Test Case 19.2: Test that nested heap filters are solved.
SELECT
  products.id
FROM products
WHERE
  (products.id @@@ paradedb.all())
  AND (products.name ILIKE ANY (array['%Socks%']))
  AND (products.created_at < (SELECT created_at FROM products WHERE products.id = 7) OR products.id < 7 AND products.created_at = (SELECT created_at FROM products WHERE products.id = 7))
ORDER BY products.created_at DESC, products.id DESC
LIMIT 100;

-- Test Case 19.3: Test that subqueries on the RHS of a heap filter which match somethin
-- result in an error.
SELECT
  products.id
FROM products
WHERE
  (products.id @@@ paradedb.all())
  AND (products.name ILIKE ANY (array['%Nike%', '%Adidas%']))
  AND (products.created_at < (SELECT created_at FROM products WHERE products.id = 8) OR products.id < 8 AND products.created_at = (SELECT created_at FROM products WHERE products.id = 8))
ORDER BY products.created_at DESC, products.id DESC
LIMIT 100;

-- Test Case 19.4: Test that nested heap filters are solved (with non-empty result).
SELECT
  products.id
FROM products
WHERE
  (products.id @@@ paradedb.all())
  AND (products.name ILIKE ANY (array['%Apple%', '%Samsung%']))
  AND (products.created_at < (SELECT created_at FROM products WHERE products.id = 8) OR products.id < 8 AND products.created_at = (SELECT created_at FROM products WHERE products.id = 8))
ORDER BY products.created_at DESC, products.id DESC
LIMIT 100;

-- Test Case 19.5: Test that subqueries returning no results are handled correctly.
-- This subquery deliberately uses a non-existent product ID (99999) to ensure empty results.
SELECT
  products.id
FROM products
WHERE
  (products.id @@@ paradedb.all())
  AND (products.name ILIKE ANY (array['%Apple%', '%Samsung%']))
  AND (products.created_at < (SELECT created_at FROM products WHERE products.id = 99999) OR products.id < 5)
ORDER BY products.created_at DESC, products.id DESC
LIMIT 100;

-- Test Case 19.6: Test multiple subqueries where one returns empty results.
-- Uses both an existing ID (8) and a non-existent ID (88888) in different subqueries.
SELECT
  products.id
FROM products
WHERE
  (products.id @@@ paradedb.all())
  AND (products.category_id = (SELECT category_id FROM products WHERE products.id = 8))
  AND (products.description NOT LIKE (SELECT description FROM products WHERE products.id = 88888))
ORDER BY products.created_at DESC, products.id DESC
LIMIT 100;

RESET paradedb.enable_filter_pushdown;
