-- Test for aggregate function columns in GROUP BY clause limitation
-- This tests the specific case where Tantivy cannot handle aggregate fields in GROUP BY

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan = ON;

-- Create test table
CREATE TABLE groupby_conflict_test (
    id SERIAL PRIMARY KEY,
    title TEXT,
    category TEXT,
    rating INTEGER,
    price FLOAT,
    views INTEGER
);

-- Insert deterministic test data
INSERT INTO groupby_conflict_test (title, category, rating, price, views) VALUES
-- Various ratings to test GROUP BY on rating field
('Product A1', 'electronics', 1, 100.00, 500),
('Product A2', 'electronics', 1, 150.00, 600),
('Product B1', 'electronics', 2, 200.00, 700),
('Product B2', 'electronics', 2, 250.00, 800),
('Product C1', 'books', 3, 30.00, 300),
('Product C2', 'books', 3, 40.00, 400),
('Product D1', 'books', 4, 50.00, 450),
('Product D2', 'books', 4, 60.00, 500),
('Product E1', 'clothing', 5, 80.00, 200),
('Product E2', 'clothing', 5, 90.00, 250),
-- More data with different price points for GROUP BY price tests
('Product F1', 'electronics', 3, 299.99, 1000),
('Product F2', 'electronics', 4, 299.99, 1100),
('Product G1', 'books', 2, 299.99, 800),
('Product G2', 'clothing', 1, 299.99, 300);

-- Create BM25 index with fast fields
CREATE INDEX groupby_conflict_idx ON groupby_conflict_test 
USING bm25(id, title, category, rating, price, views)
WITH (
    key_field='id',
    text_fields='{"title": {}, "category": {"fast": true}}',
    numeric_fields='{"rating": {"fast": true}, "price": {"fast": true}, "views": {"fast": true}}'
);

-- =====================================================================
-- Test 1: GROUP BY on rating field with AVG(rating)
-- =====================================================================
EXPLAIN (VERBOSE, COSTS OFF)
SELECT rating, AVG(rating) as avg_rating, COUNT(*) as count
FROM groupby_conflict_test
WHERE title @@@ 'Product'
GROUP BY rating
ORDER BY rating;

-- Execute the query
SELECT rating, AVG(rating) as avg_rating, COUNT(*) as count
FROM groupby_conflict_test
WHERE title @@@ 'Product'
GROUP BY rating
ORDER BY rating;

-- =====================================================================
-- Test 2: GROUP BY on price field with SUM(price)
-- =====================================================================
EXPLAIN (VERBOSE, COSTS OFF)
SELECT price, SUM(price) as total_price, COUNT(*) as count
FROM groupby_conflict_test
WHERE title @@@ 'Product'
GROUP BY price
ORDER BY price;

-- Execute the query
SELECT price, SUM(price) as total_price, COUNT(*) as count
FROM groupby_conflict_test
WHERE title @@@ 'Product'
GROUP BY price
ORDER BY price;

-- =====================================================================
-- Test 3: GROUP BY on views field with MAX(views)
-- =====================================================================
EXPLAIN (VERBOSE, COSTS OFF)
SELECT views, MAX(views) as max_views, MIN(views) as min_views
FROM groupby_conflict_test
WHERE title @@@ 'Product'
GROUP BY views
ORDER BY views;

-- Execute the query
SELECT views, MAX(views) as max_views, MIN(views) as min_views
FROM groupby_conflict_test
WHERE title @@@ 'Product'
GROUP BY views
ORDER BY views;

-- =====================================================================
-- Test 4: Multiple aggregate functions on same field as GROUP BY
-- =====================================================================
-- This should NOT use AggregateScan: rating used in both GROUP BY and multiple aggregates
EXPLAIN (VERBOSE, COSTS OFF)
SELECT rating, 
       AVG(rating) as avg_rating,
       MIN(rating) as min_rating, 
       MAX(rating) as max_rating,
       COUNT(*) as count
FROM groupby_conflict_test
WHERE category @@@ 'electronics'
GROUP BY rating
ORDER BY rating;

-- Execute the query
SELECT rating, 
       AVG(rating) as avg_rating,
       MIN(rating) as min_rating, 
       MAX(rating) as max_rating,
       COUNT(*) as count
FROM groupby_conflict_test
WHERE category @@@ 'electronics'
GROUP BY rating
ORDER BY rating;

-- =====================================================================
-- Edge case: GROUP BY on non-fast field with aggregates on fast fields
-- =====================================================================

-- Test 7: GROUP BY on title (not fast field) - should fall back anyway
EXPLAIN (VERBOSE, COSTS OFF)
SELECT title,
       AVG(rating) as avg_rating,
       COUNT(*) as count
FROM groupby_conflict_test
WHERE category @@@ 'electronics'
GROUP BY title
ORDER BY title;

-- Execute the query
SELECT title,
       AVG(rating) as avg_rating,
       COUNT(*) as count
FROM groupby_conflict_test
WHERE category @@@ 'electronics'
GROUP BY title
ORDER BY title
LIMIT 5;

-- Clean up
DROP TABLE groupby_conflict_test;
