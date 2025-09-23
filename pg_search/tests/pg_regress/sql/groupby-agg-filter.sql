-- Test for GROUP BY, aggregates, and FILTER clauses
-- This test covers all aspects of ParadeDB's FILTER support with @@@ operators
-- Tests both optimized AggregateScan execution and fallback scenarios

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan = ON;

-- Create filter_agg_test test table
DROP TABLE IF EXISTS filter_agg_test CASCADE;
CREATE TABLE filter_agg_test (
    id SERIAL PRIMARY KEY,
    title TEXT,
    description TEXT,
    category TEXT,
    brand TEXT,
    status TEXT,
    price NUMERIC,
    rating INTEGER,
    in_stock BOOLEAN,
    views INTEGER
);

-- Insert deterministic test data covering all scenarios
INSERT INTO filter_agg_test (title, description, category, brand, status, price, rating, in_stock, views) VALUES
-- Electronics (Apple)
('MacBook Pro', 'laptop computer with keyboard', 'electronics', 'Apple', 'available', 2499.99, 5, true, 1500),
('iMac Desktop', 'desktop computer with monitor', 'electronics', 'Apple', 'available', 1999.99, 5, true, 1200),
('iPad Tablet', 'tablet with stylus', 'electronics', 'Apple', 'sold', 899.99, 4, false, 2000),
-- Electronics (Samsung)
('Galaxy Laptop', 'laptop computer gaming', 'electronics', 'Samsung', 'available', 1799.99, 4, true, 800),
('Samsung Monitor', 'monitor ultra wide', 'electronics', 'Samsung', 'available', 599.99, 4, true, 600),
('Galaxy Tablet', 'tablet android device', 'electronics', 'Samsung', 'sold', 649.99, 3, false, 900),
-- Electronics (Generic)
('Gaming Keyboard', 'keyboard mechanical gaming', 'electronics', 'Generic', 'available', 149.99, 3, true, 400),
('Wireless Mouse', 'mouse wireless pro', 'electronics', 'Generic', 'available', 79.99, 4, true, 300),
-- Clothing
('Developer T-Shirt', 'shirt for programming', 'clothing', 'TechWear', 'available', 24.99, 4, true, 200),
('Database Hoodie', 'hoodie with logo', 'clothing', 'TechWear', 'available', 59.99, 5, true, 350),
('Running Shoes', 'shoes for running', 'clothing', 'SportsBrand', 'sold', 129.99, 4, false, 180),
('Casual Jeans', 'jeans casual wear', 'clothing', 'FashionCo', 'available', 79.99, 3, true, 120),
-- Books
('Database Systems', 'database design book', 'books', 'TechPress', 'available', 49.99, 5, true, 1800),
('Search Engines', 'search engine design', 'books', 'TechPress', 'available', 59.99, 5, true, 1600),
('SQL Performance', 'sql optimization guide', 'books', 'DataBooks', 'sold', 39.99, 4, false, 1400),
('PostgreSQL Guide', 'postgresql advanced topics', 'books', 'DataBooks', 'available', 44.99, 4, true, 1200),
-- Sports
('Tennis Racket', 'racket for tennis', 'sports', 'SportsCorp', 'available', 199.99, 4, true, 250),
('Basketball', 'basketball official size', 'sports', 'SportsCorp', 'available', 29.99, 3, true, 150),
('Soccer Ball', 'soccer ball professional', 'sports', 'PlayTime', 'sold', 39.99, 4, false, 200),
('Golf Clubs', 'golf club set premium', 'sports', 'GolfPro', 'available', 899.99, 5, true, 100);

-- Create BM25 index with fast fields for all aggregation scenarios
CREATE INDEX filter_agg_idx ON filter_agg_test
USING bm25(id, title, description, category, brand, status, price, rating, in_stock, views)
WITH (
    key_field='id',
    text_fields='{
        "title": {},
        "description": {},
        "category": {"fast": true},
        "brand": {"fast": true},
        "status": {"fast": true}
    }',
    numeric_fields='{
        "price": {"fast": true},
        "rating": {"fast": true},
        "views": {"fast": true}
    }',
    boolean_fields='{
        "in_stock": {"fast": true}
    }'
);

-- =====================================================================
-- SECTION 1: Basic FILTER clause tests (no GROUP BY)
-- =====================================================================

-- Test 1.1: Single FILTER with @@@ (should use AggregateScan)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE description @@@ 'laptop') AS laptop_count
FROM filter_agg_test;

SELECT 
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE description @@@ 'laptop') AS laptop_count
FROM filter_agg_test;

-- Test 1.2: Multiple FILTER clauses (should use optimized multi-query)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE description @@@ 'laptop') AS laptop_count,
    COUNT(*) FILTER (WHERE description @@@ 'keyboard') AS keyboard_count,
    COUNT(*) FILTER (WHERE category @@@ 'books') AS books_count
FROM filter_agg_test;

SELECT 
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE description @@@ 'laptop') AS laptop_count,
    COUNT(*) FILTER (WHERE description @@@ 'keyboard') AS keyboard_count,
    COUNT(*) FILTER (WHERE category @@@ 'books') AS books_count
FROM filter_agg_test;

-- Test 1.3: FILTER with base WHERE clause
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) AS available_total,
    COUNT(*) FILTER (WHERE category @@@ 'electronics') AS electronics_available,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_available
FROM filter_agg_test
WHERE status @@@ 'available';

SELECT 
    COUNT(*) AS available_total,
    COUNT(*) FILTER (WHERE category @@@ 'electronics') AS electronics_available,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_available
FROM filter_agg_test
WHERE status @@@ 'available';

-- Test 1.4: Multiple aggregate types with FILTER
SELECT 
    COUNT(*) AS total,
    SUM(price) FILTER (WHERE category @@@ 'electronics') AS electronics_revenue,
    AVG(rating) FILTER (WHERE brand @@@ 'Apple') AS apple_avg_rating,
    MAX(price) FILTER (WHERE description @@@ 'laptop') AS max_laptop_price,
    MIN(views) FILTER (WHERE status @@@ 'sold') AS min_sold_views
FROM filter_agg_test;

-- =====================================================================
-- SECTION 2: FILTER optimization scenarios
-- =====================================================================

-- Test 2.1: All aggregates have NO filters (single query optimization)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) AS total,
    SUM(price) AS total_revenue,
    AVG(rating) AS avg_rating,
    MAX(views) AS max_views
FROM filter_agg_test
WHERE status @@@ 'available';

-- Test 2.2: All aggregates have SAME filter (single query optimization)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) FILTER (WHERE category @@@ 'electronics') AS electronics_count,
    SUM(price) FILTER (WHERE category @@@ 'electronics') AS electronics_revenue,
    AVG(rating) FILTER (WHERE category @@@ 'electronics') AS electronics_avg_rating
FROM filter_agg_test;

-- Test 2.3: Mixed filters - some same, some different (partial optimization)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) AS total,                                           -- No filter
    SUM(price) AS total_revenue,                                -- No filter
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count,  -- Filter 1
    SUM(price) FILTER (WHERE brand @@@ 'Apple') AS apple_revenue, -- Filter 1 (same)
    COUNT(*) FILTER (WHERE category @@@ 'books') AS books_count   -- Filter 2 (different)
FROM filter_agg_test;

-- Test 2.4: Many different filters (multi-query required)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) FILTER (WHERE category @@@ 'electronics') AS electronics,
    COUNT(*) FILTER (WHERE category @@@ 'clothing') AS clothing,
    COUNT(*) FILTER (WHERE category @@@ 'books') AS books,
    COUNT(*) FILTER (WHERE category @@@ 'sports') AS sports,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple,
    COUNT(*) FILTER (WHERE status @@@ 'sold') AS sold,
    COUNT(*) FILTER (WHERE rating >= 4) AS highly_rated,
    COUNT(*) FILTER (WHERE in_stock = true) AS in_stock_items
FROM filter_agg_test;

-- =====================================================================
-- SECTION 3: GROUP BY with FILTER clauses
-- =====================================================================

-- Test 3.1: Simple GROUP BY with single FILTER
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    category,
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count
FROM filter_agg_test
GROUP BY category
ORDER BY category;

SELECT 
    category,
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- Test 3.2: GROUP BY with multiple different FILTER clauses
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    category,
    COUNT(*) FILTER (WHERE status @@@ 'available') AS available_count,
    COUNT(*) FILTER (WHERE rating >= 4) AS highly_rated_count,
    AVG(price) FILTER (WHERE in_stock = true) AS avg_available_price
FROM filter_agg_test
GROUP BY category
ORDER BY category;

SELECT 
    category,
    COUNT(*) FILTER (WHERE status @@@ 'available') AS available_count,
    COUNT(*) FILTER (WHERE rating >= 4) AS highly_rated_count,
    AVG(price) FILTER (WHERE in_stock = true) AS avg_available_price
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- Test 3.3: GROUP BY with mixed aggregates (some filtered, some not)
SELECT 
    brand,
    COUNT(*) AS total_products,
    AVG(price) AS avg_price,
    COUNT(*) FILTER (WHERE category @@@ 'electronics') AS electronics_count,
    SUM(price) FILTER (WHERE status @@@ 'available') AS available_revenue
FROM filter_agg_test
WHERE brand @@@ 'Apple OR Samsung OR TechPress'
GROUP BY brand
ORDER BY brand;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    brand,
    COUNT(*) AS total_products,
    AVG(price) AS avg_price,
    COUNT(*) FILTER (WHERE category @@@ 'electronics') AS electronics_count,
    SUM(price) FILTER (WHERE status @@@ 'available') AS available_revenue
FROM filter_agg_test
WHERE brand @@@ 'Apple OR Samsung OR TechPress'
GROUP BY brand
ORDER BY brand;

-- Test 3.4: Multi-column GROUP BY with FILTER
SELECT 
    category,
    status,
    COUNT(*) AS count,
    AVG(rating) FILTER (WHERE price > 100) AS avg_rating_expensive,
    MAX(views) FILTER (WHERE in_stock = true) AS max_views_available
FROM filter_agg_test
GROUP BY category, status
ORDER BY category, status;

-- =====================================================================
-- SECTION 4: Complex FILTER conditions
-- =====================================================================

-- Test 4.1: Boolean AND in FILTER
SELECT 
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE description @@@ 'laptop' AND price > 1000) AS expensive_laptops,
    COUNT(*) FILTER (WHERE category @@@ 'electronics' AND brand @@@ 'Apple') AS apple_electronics
FROM filter_agg_test;

-- Test 4.2: Boolean OR in FILTER
SELECT 
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE category @@@ 'books' OR category @@@ 'sports') AS books_or_sports,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple' OR brand @@@ 'Samsung') AS major_brands
FROM filter_agg_test;

-- Test 4.3: Complex nested boolean expressions
SELECT 
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE (category @@@ 'electronics' AND price > 500) OR (category @@@ 'books' AND rating >= 4)) AS complex_filter
FROM filter_agg_test;

-- =====================================================================
-- SECTION 5: Edge cases and error conditions
-- =====================================================================

-- Test 5.1: FILTER with non-@@@ conditions
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE in_stock = true) AS expensive_items,
    COUNT(*) FILTER (WHERE category @@@ 'electronics') AS electronics
FROM filter_agg_test;

-- Test 5.2: Empty result sets
SELECT 
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE description @@@ 'nonexistent_term_xyz') AS no_matches,
    COUNT(*) FILTER (WHERE price > 10000) AS too_expensive
FROM filter_agg_test;

-- Test 5.3: NULL handling (add some NULL values first)
UPDATE filter_agg_test SET description = NULL WHERE id % 7 = 0;

SELECT 
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE description IS NULL) AS null_descriptions,
    COUNT(*) FILTER (WHERE description @@@ 'laptop') AS laptop_count,
    COUNT(*) FILTER (WHERE description IS NOT NULL AND description @@@ 'laptop') AS laptop_not_null
FROM filter_agg_test;

-- Test 5.4: Unsupported aggregate functions (should fall back)
SELECT 
    COUNT(*) AS total,
    STDDEV(price) FILTER (WHERE category @@@ 'electronics') AS price_stddev,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count
FROM filter_agg_test;

-- =====================================================================
-- SECTION 6: Performance and stress tests
-- =====================================================================

-- Test 6.1: Large number of FILTER clauses (stress test)
SELECT 
    COUNT(*) FILTER (WHERE category @@@ 'electronics') AS f1,
    COUNT(*) FILTER (WHERE category @@@ 'clothing') AS f2,
    COUNT(*) FILTER (WHERE category @@@ 'books') AS f3,
    COUNT(*) FILTER (WHERE category @@@ 'sports') AS f4,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS f5,
    COUNT(*) FILTER (WHERE brand @@@ 'Samsung') AS f6,
    COUNT(*) FILTER (WHERE brand @@@ 'TechPress') AS f7,
    COUNT(*) FILTER (WHERE status @@@ 'available') AS f8,
    COUNT(*) FILTER (WHERE status @@@ 'sold') AS f9,
    COUNT(*) FILTER (WHERE rating >= 4) AS f10,
    COUNT(*) FILTER (WHERE rating >= 5) AS f11,
    COUNT(*) FILTER (WHERE in_stock = true) AS f12
FROM filter_agg_test;

-- Test 6.2: Performance comparison - separate queries vs FILTER
-- Separate queries (slower approach)
SELECT COUNT(*) FROM filter_agg_test WHERE description @@@ 'laptop';
SELECT COUNT(*) FROM filter_agg_test WHERE description @@@ 'keyboard';
SELECT COUNT(*) FROM filter_agg_test WHERE category @@@ 'books';

-- Single query with FILTER (optimized approach)
SELECT 
    COUNT(*) FILTER (WHERE description @@@ 'laptop') AS laptop_count,
    COUNT(*) FILTER (WHERE description @@@ 'keyboard') AS keyboard_count,
    COUNT(*) FILTER (WHERE category @@@ 'books') AS books_count
FROM filter_agg_test;

-- =====================================================================
-- SECTION 7: Comparison with direct paradedb.aggregate calls
-- =====================================================================

-- Test 7.1: Direct aggregate call for comparison
SELECT paradedb.aggregate(
    index => 'filter_agg_idx'::regclass,
    query => paradedb.parse('category:electronics'),
    agg => '{
        "total_count": {"value_count": {"field": "id"}},
        "avg_price": {"avg": {"field": "price"}},
        "max_rating": {"max": {"field": "rating"}}
    }'::json
);

-- Test 7.2: Complex aggregation with grouping
SELECT paradedb.aggregate(
    index => 'filter_agg_idx'::regclass,
    query => paradedb.parse('status:available'),
    agg => '{
        "category_breakdown": {
            "terms": {
                "field": "category",
                "size": 10,
                "order": {"_key": "asc"}
            },
            "aggs": {
                "avg_price": {"avg": {"field": "price"}},
                "total_views": {"sum": {"field": "views"}}
            }
        }
    }'::json
);

-- =====================================================================
-- SECTION 8: Verify ORDER BY preservation in GROUP BY + FILTER
-- =====================================================================

-- Test 8.1: ORDER BY with GROUP BY and FILTER (verify deterministic sorting)
SELECT 
    category,
    COUNT(*) FILTER (WHERE status @@@ 'available') AS available_count,
    COUNT(*) FILTER (WHERE rating >= 4) AS highly_rated_count
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- Test 8.2: ORDER BY aggregate result (should fall back)
SELECT 
    category,
    COUNT(*) AS total,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count
FROM filter_agg_test
GROUP BY category
ORDER BY apple_count DESC;

-- =====================================================================
-- SECTION 9: Limitations and fallback scenarios
-- =====================================================================

-- Test 9.1: COUNT(DISTINCT) with FILTER (should fall back)
SELECT 
    COUNT(DISTINCT category) AS unique_categories,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count
FROM filter_agg_test;

-- Test 9.2: Window functions (should fall back)
SELECT 
    category,
    price,
    COUNT(*) OVER() AS total_count,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') OVER() AS apple_count_window
FROM filter_agg_test
WHERE category @@@ 'electronics'
ORDER BY price DESC
LIMIT 5;

-- Test 9.3: Complex aggregation patterns (avoiding subqueries that may cause issues)
SELECT 
    category,
    COUNT(*) AS total_in_category,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_in_category
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- Clean up
DROP TABLE filter_agg_test CASCADE;
