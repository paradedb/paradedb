-- Test for GROUP BY, aggregates, and FILTER clauses
-- This test covers all aspects of ParadeDB's FILTER support with @@@ operators
-- Tests both optimized AggregateScan execution and fallback scenarios

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan = ON;

-- Create filter_agg_test test table
DROP TABLE IF EXISTS filter_agg_test CASCADE;
CREATE TABLE filter_agg_test (
    id INT PRIMARY KEY,
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
INSERT INTO filter_agg_test (id, title, description, category, brand, status, price, rating, in_stock, views) VALUES
-- Electronics (Apple)
(1, 'MacBook Pro', 'laptop computer with keyboard', 'electronics', 'Apple', 'available', 2499.99, 5, true, 1500),
(2, 'iMac Desktop', 'desktop computer with monitor', 'electronics', 'Apple', 'available', 1999.99, 5, true, 1200),
(3, 'iPad Tablet', 'tablet with stylus', 'electronics', 'Apple', 'sold', 899.99, 4, false, 2000),
-- Electronics (Samsung)
(4, 'Galaxy Laptop', 'laptop computer gaming', 'electronics', 'Samsung', 'available', 1799.99, 4, true, 800),
(5, 'Samsung Monitor', 'monitor ultra wide', 'electronics', 'Samsung', 'available', 599.99, 4, true, 600),
(6, 'Galaxy Tablet', 'tablet android device', 'electronics', 'Samsung', 'sold', 649.99, 3, false, 900),
-- Electronics (Generic)
(7, 'Gaming Keyboard', 'keyboard mechanical gaming', 'electronics', 'Generic', 'available', 149.99, 3, true, 400),
(8, 'Wireless Mouse', 'mouse wireless pro', 'electronics', 'Generic', 'available', 79.99, 4, true, 300),
-- Clothing
(9, 'Developer T-Shirt', 'shirt for programming', 'clothing', 'TechWear', 'available', 24.99, 4, true, 200),
(10, 'Database Hoodie', 'hoodie with logo', 'clothing', 'TechWear', 'available', 59.99, 5, true, 350),
(11, 'Running Shoes', 'shoes for running', 'clothing', 'SportsBrand', 'sold', 129.99, 4, false, 180),
(12, 'Casual Jeans', 'jeans casual wear', 'clothing', 'FashionCo', 'available', 79.99, 3, true, 120),
-- Books
(13, 'Database Systems', 'database design book', 'books', 'TechPress', 'available', 49.99, 5, true, 1800),
(14, 'Search Engines', 'search engine design', 'books', 'TechPress', 'available', 59.99, 5, true, 1600),
(15, 'SQL Performance', 'sql optimization guide', 'books', 'DataBooks', 'sold', 39.99, 4, false, 1400),
(16, 'PostgreSQL Guide', 'postgresql advanced topics', 'books', 'DataBooks', 'available', 44.99, 4, true, 1200),
-- Sports
(17, 'Tennis Racket', 'racket for tennis', 'sports', 'SportsCorp', 'available', 199.99, 4, true, 250),
(18, 'Basketball', 'basketball official size', 'sports', 'SportsCorp', 'available', 29.99, 3, true, 150),
(19, 'Soccer Ball', 'soccer ball professional', 'sports', 'PlayTime', 'sold', 39.99, 4, false, 200),
(20, 'Golf Clubs', 'golf club set premium', 'sports', 'GolfPro', 'available', 899.99, 5, true, 100);

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

-- Test 1.2: Multiple FILTER clauses (should use multi-query)
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
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    category,
    status,
    COUNT(*) AS count,
    AVG(rating) FILTER (WHERE price > 100) AS avg_rating_expensive,
    MAX(views) FILTER (WHERE in_stock = false) AS max_views_out_of_stock,
    MAX(views) FILTER (WHERE status @@@ 'available') AS max_views_available
FROM filter_agg_test
GROUP BY category, status
ORDER BY category, status;

SELECT 
    category,
    status,
    COUNT(*) AS count,
    AVG(rating) FILTER (WHERE price > 100) AS avg_rating_expensive,
    MAX(views) FILTER (WHERE in_stock = false) AS max_views_out_of_stock,
    MAX(views) FILTER (WHERE status @@@ 'available') AS max_views_available
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
        "sum_price": {"sum": {"field": "price"}},
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
-- SECTION 9: Multiple Aggregates with Same Filter (MultiCollector Optimization)
-- =====================================================================

-- Test 9.1: Multiple aggregates with same filter - no GROUP BY
-- This should trigger MultiCollector optimization
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count,
    SUM(price) FILTER (WHERE brand @@@ 'Apple') AS apple_total_price,
    AVG(price) FILTER (WHERE brand @@@ 'Apple') AS apple_avg_price,
    MIN(price) FILTER (WHERE brand @@@ 'Apple') AS apple_min_price,
    MAX(price) FILTER (WHERE brand @@@ 'Apple') AS apple_max_price
FROM filter_agg_test;

SELECT 
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count,
    SUM(price) FILTER (WHERE brand @@@ 'Apple') AS apple_total_price,
    AVG(price) FILTER (WHERE brand @@@ 'Apple') AS apple_avg_price,
    MIN(price) FILTER (WHERE brand @@@ 'Apple') AS apple_min_price,
    MAX(price) FILTER (WHERE brand @@@ 'Apple') AS apple_max_price
FROM filter_agg_test;

-- Test 9.2: Multiple aggregates with same filter - with GROUP BY
-- This should trigger MultiCollector optimization within each group
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    category,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count,
    SUM(price) FILTER (WHERE brand @@@ 'Apple') AS apple_total_price,
    AVG(price) FILTER (WHERE brand @@@ 'Apple') AS apple_avg_price,
    MIN(rating) FILTER (WHERE brand @@@ 'Apple') AS apple_min_rating,
    MAX(views) FILTER (WHERE brand @@@ 'Apple') AS apple_max_views
FROM filter_agg_test
GROUP BY category
ORDER BY category;

SELECT 
    category,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count,
    SUM(price) FILTER (WHERE brand @@@ 'Apple') AS apple_total_price,
    AVG(price) FILTER (WHERE brand @@@ 'Apple') AS apple_avg_price,
    MIN(rating) FILTER (WHERE brand @@@ 'Apple') AS apple_min_rating,
    MAX(views) FILTER (WHERE brand @@@ 'Apple') AS apple_max_views
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- Test 9.3: Multiple aggregates with same complex filter
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    category,
    COUNT(*) FILTER (WHERE status @@@ 'available' AND price > 500) AS expensive_available_count,
    SUM(price) FILTER (WHERE status @@@ 'available' AND price > 500) AS expensive_available_total,
    AVG(rating) FILTER (WHERE status @@@ 'available' AND price > 500) AS expensive_available_avg_rating
FROM filter_agg_test
GROUP BY category
ORDER BY category;

SELECT 
    category,
    COUNT(*) FILTER (WHERE status @@@ 'available' AND price > 500) AS expensive_available_count,
    SUM(price) FILTER (WHERE status @@@ 'available' AND price > 500) AS expensive_available_total,
    AVG(rating) FILTER (WHERE status @@@ 'available' AND price > 500) AS expensive_available_avg_rating
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- Test 9.4: Multiple aggregates with same numeric filter
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    category,
    COUNT(*) FILTER (WHERE rating >= 4) AS highly_rated_count,
    SUM(price) FILTER (WHERE rating >= 4) AS highly_rated_total_price,
    AVG(price) FILTER (WHERE rating >= 4) AS highly_rated_avg_price,
    MIN(price) FILTER (WHERE rating >= 4) AS highly_rated_min_price,
    MAX(price) FILTER (WHERE rating >= 4) AS highly_rated_max_price
FROM filter_agg_test
GROUP BY category
ORDER BY category;

SELECT 
    category,
    COUNT(*) FILTER (WHERE rating >= 4) AS highly_rated_count,
    SUM(price) FILTER (WHERE rating >= 4) AS highly_rated_total_price,
    AVG(price) FILTER (WHERE rating >= 4) AS highly_rated_avg_price,
    MIN(price) FILTER (WHERE rating >= 4) AS highly_rated_min_price,
    MAX(price) FILTER (WHERE rating >= 4) AS highly_rated_max_price
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- Test 9.5: Multiple aggregates with same boolean filter
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    brand,
    COUNT(*) FILTER (WHERE in_stock = true) AS in_stock_count,
    SUM(price) FILTER (WHERE in_stock = true) AS in_stock_total_price,
    AVG(rating) FILTER (WHERE in_stock = true) AS in_stock_avg_rating,
    MAX(views) FILTER (WHERE in_stock = true) AS in_stock_max_views
FROM filter_agg_test
GROUP BY brand
ORDER BY brand;

SELECT 
    brand,
    COUNT(*) FILTER (WHERE in_stock = true) AS in_stock_count,
    SUM(price) FILTER (WHERE in_stock = true) AS in_stock_total_price,
    AVG(rating) FILTER (WHERE in_stock = true) AS in_stock_avg_rating,
    MAX(views) FILTER (WHERE in_stock = true) AS in_stock_max_views
FROM filter_agg_test
GROUP BY brand
ORDER BY brand;

-- Test 9.6: Mix of same and different filters (should optimize same-filter groups)
SELECT 
    category,
    -- These three should use MultiCollector (same filter)
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count,
    SUM(price) FILTER (WHERE brand @@@ 'Apple') AS apple_total_price,
    AVG(rating) FILTER (WHERE brand @@@ 'Apple') AS apple_avg_rating,
    -- These two should use MultiCollector (same filter, different from above)
    COUNT(*) FILTER (WHERE status @@@ 'available') AS available_count,
    MAX(views) FILTER (WHERE status @@@ 'available') AS available_max_views,
    -- This one is different (separate query)
    MIN(price) FILTER (WHERE rating >= 4) AS highly_rated_min_price
FROM filter_agg_test
GROUP BY category
ORDER BY category;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    category,
    -- These three should use MultiCollector (same filter)
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count,
    SUM(price) FILTER (WHERE brand @@@ 'Apple') AS apple_total_price,
    AVG(rating) FILTER (WHERE brand @@@ 'Apple') AS apple_avg_rating,
    -- These two should use MultiCollector (same filter, different from above)
    COUNT(*) FILTER (WHERE status @@@ 'available') AS available_count,
    MAX(views) FILTER (WHERE status @@@ 'available') AS available_max_views,
    -- This one is different (separate query)
    MIN(price) FILTER (WHERE rating >= 4) AS highly_rated_min_price
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- Test 9.7: Many aggregates with same filter (stress test for MultiCollector)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    COUNT(*) FILTER (WHERE brand @@@ 'Samsung') AS samsung_count,
    SUM(price) FILTER (WHERE brand @@@ 'Samsung') AS samsung_total_price,
    MIN(price) FILTER (WHERE brand @@@ 'Samsung') AS samsung_min_price,
    MAX(price) FILTER (WHERE brand @@@ 'Samsung') AS samsung_max_price,
    MIN(rating) FILTER (WHERE brand @@@ 'Samsung') AS samsung_min_rating,
    MAX(rating) FILTER (WHERE brand @@@ 'Samsung') AS samsung_max_rating,
    SUM(views) FILTER (WHERE brand @@@ 'Samsung') AS samsung_total_views
FROM filter_agg_test;

SELECT 
    COUNT(*) FILTER (WHERE brand @@@ 'Samsung') AS samsung_count,
    SUM(price) FILTER (WHERE brand @@@ 'Samsung') AS samsung_total_price,
    MIN(price) FILTER (WHERE brand @@@ 'Samsung') AS samsung_min_price,
    MAX(price) FILTER (WHERE brand @@@ 'Samsung') AS samsung_max_price,
    MIN(rating) FILTER (WHERE brand @@@ 'Samsung') AS samsung_min_rating,
    MAX(rating) FILTER (WHERE brand @@@ 'Samsung') AS samsung_max_rating,
    SUM(views) FILTER (WHERE brand @@@ 'Samsung') AS samsung_total_views
FROM filter_agg_test;

-- Test 9.8: Multiple aggregates with same filter on different field types
SELECT 
    category,
    COUNT(*) FILTER (WHERE price > 1000) AS expensive_count,
    SUM(rating) FILTER (WHERE price > 1000) AS expensive_rating_sum,
    AVG(views) FILTER (WHERE price > 1000) AS expensive_avg_views,
    MIN(price) FILTER (WHERE price > 1000) AS expensive_min_price,
    MAX(price) FILTER (WHERE price > 1000) AS expensive_max_price
FROM filter_agg_test
GROUP BY category
ORDER BY category;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    category,
    COUNT(*) FILTER (WHERE price > 1000) AS expensive_count,
    SUM(rating) FILTER (WHERE price > 1000) AS expensive_rating_sum,
    AVG(views) FILTER (WHERE price > 1000) AS expensive_avg_views,
    MIN(price) FILTER (WHERE price > 1000) AS expensive_min_price,
    MAX(price) FILTER (WHERE price > 1000) AS expensive_max_price
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- Test 9.9: Same filter with multi-column GROUP BY
SELECT 
    category,
    brand,
    COUNT(*) FILTER (WHERE status @@@ 'available') AS available_count,
    SUM(price) FILTER (WHERE status @@@ 'available') AS available_total_price,
    SUM(rating) FILTER (WHERE status @@@ 'available') AS available_sum_rating
FROM filter_agg_test
GROUP BY category, brand
ORDER BY category, brand;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    category,
    brand,
    COUNT(*) FILTER (WHERE status @@@ 'available') AS available_count,
    SUM(price) FILTER (WHERE status @@@ 'available') AS available_total_price,
    SUM(rating) FILTER (WHERE status @@@ 'available') AS available_sum_rating
FROM filter_agg_test
GROUP BY category, brand
ORDER BY category, brand;

-- Test 9.10: Identical filters with different aggregate functions on same field
SELECT 
    category,
    SUM(price) FILTER (WHERE brand @@@ 'Apple') AS apple_price_sum,
    AVG(price) FILTER (WHERE brand @@@ 'Apple') AS apple_price_avg,
    MIN(price) FILTER (WHERE brand @@@ 'Apple') AS apple_price_min,
    MAX(price) FILTER (WHERE brand @@@ 'Apple') AS apple_price_max,
    COUNT(price) FILTER (WHERE brand @@@ 'Apple') AS apple_price_count
FROM filter_agg_test
GROUP BY category
ORDER BY category;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    category,
    SUM(price) FILTER (WHERE brand @@@ 'Apple') AS apple_price_sum,
    AVG(price) FILTER (WHERE brand @@@ 'Apple') AS apple_price_avg,
    MIN(price) FILTER (WHERE brand @@@ 'Apple') AS apple_price_min,
    MAX(price) FILTER (WHERE brand @@@ 'Apple') AS apple_price_max,
    COUNT(price) FILTER (WHERE brand @@@ 'Apple') AS apple_price_count
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- =====================================================================
-- SECTION 10: Limitations and fallback scenarios
-- =====================================================================

-- Test 10.1: COUNT(DISTINCT) with FILTER (should fall back)
SELECT 
    COUNT(DISTINCT category) AS unique_categories,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count
FROM filter_agg_test;

-- Test 10.2: Window functions (should fall back)
SELECT 
    category,
    price,
    COUNT(*) OVER() AS total_count,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') OVER() AS apple_count_window
FROM filter_agg_test
WHERE category @@@ 'electronics'
ORDER BY price DESC
LIMIT 5;

-- Test 10.3: Complex aggregation patterns (avoiding subqueries that may cause issues)
SELECT 
    category,
    COUNT(*) AS total_in_category,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_in_category
FROM filter_agg_test
GROUP BY category
ORDER BY category;

-- Test 10.4: GROUP BY with FILTER without @@@ (should fall back)
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
-- SECTION 11: WHERE clause + GROUP BY + FILTER (testing sentinel)
-- =====================================================================

-- Test 11.1: WHERE clause with GROUP BY and all aggregates filtered
-- This tests that the sentinel uses the WHERE clause, not all documents
-- Should only show electronics category (due to WHERE clause)
-- But within electronics, should show all groups even if filter doesn't match
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    brand,
    COUNT(*) FILTER (WHERE status @@@ 'sold') AS sold_count,
    SUM(price) FILTER (WHERE status @@@ 'sold') AS sold_total
FROM filter_agg_test
WHERE category @@@ 'electronics'
GROUP BY brand
ORDER BY brand;

SELECT 
    brand,
    COUNT(*) FILTER (WHERE status @@@ 'sold') AS sold_count,
    SUM(price) FILTER (WHERE status @@@ 'sold') AS sold_total
FROM filter_agg_test
WHERE category @@@ 'electronics'
GROUP BY brand
ORDER BY brand;

-- Test 11.2: WHERE clause with complex condition + GROUP BY + FILTER
-- Should only show high-priced items (price > 500) and group by category
-- All categories with expensive items should appear, even if filter doesn't match
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    category,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count,
    AVG(price) FILTER (WHERE brand @@@ 'Apple') AS apple_avg_price
FROM filter_agg_test
WHERE price > 500
GROUP BY category
ORDER BY category;

SELECT 
    category,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS apple_count,
    AVG(price) FILTER (WHERE brand @@@ 'Apple') AS apple_avg_price
FROM filter_agg_test
WHERE price > 500
GROUP BY category
ORDER BY category;

-- Clean up
DROP TABLE filter_agg_test CASCADE;
