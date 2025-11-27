-- Test TopN + Aggregates + Faceting
-- Phase 1: Basic TopN tests with window aggregate detection

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS products CASCADE;

-- Setup test data
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    category TEXT,
    brand TEXT,
    price NUMERIC,
    rating NUMERIC,
    in_stock BOOLEAN,
    sales INTEGER
);

-- Insert test data
INSERT INTO products (name, description, category, brand, price, rating, in_stock, sales) VALUES
    ('MacBook Pro', 'High-performance laptop for professionals', 'Laptops', 'Apple', 2499, 4.8, true, 150),
    ('Dell XPS 13', 'Compact and powerful ultrabook', 'Laptops', 'Dell', 1299, 4.6, true, 200),
    ('ThinkPad X1', 'Business laptop with great keyboard', 'Laptops', 'Lenovo', 1599, 4.5, true, 180),
    ('HP Spectre', 'Stylish convertible laptop', 'Laptops', 'HP', 1399, 4.4, true, 120),
    ('ASUS ROG', 'Gaming laptop with RTX graphics', 'Laptops', 'ASUS', 1899, 4.7, true, 90);

-- Create BM25 index
CREATE INDEX products_idx ON products
USING bm25(id, name, description, category, brand, price, rating, in_stock, sales)
WITH (
    key_field='id',
    text_fields='{
        "name": {},
        "description": {},
        "brand": {"fast": true}
    }',
    numeric_fields='{
        "price": {"fast": true},
        "rating": {"fast": true},
        "sales": {"fast": true}
    }',
    boolean_fields='{
        "in_stock": {"fast": true}
    }'
);

-- =============================================================================
-- OPERATOR SUPPORT TESTS
-- Testing that window aggregate pushdown works with all search operators
-- =============================================================================

-- Test 1a: Window aggregate with ||| operator (match disjunction)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description ||| 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description ||| 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 1b: Window aggregate with &&& operator (match conjunction)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description &&& 'laptop powerful'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description &&& 'laptop powerful'
ORDER BY rating DESC
LIMIT 3;

-- Test 1c: Window aggregate with === operator (term search)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description === 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description === 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 1d: Window aggregate with ### operator (phrase search)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description ### 'laptop for'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description ### 'laptop for'
ORDER BY rating DESC
LIMIT 3;

-- Test 1e: Window aggregate with proximity operator (## - any order)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description @@@ ('laptop' ## 10 ## 'powerful')
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description @@@ ('laptop' ## 10 ## 'powerful')
ORDER BY rating DESC
LIMIT 3;

-- Test 1f: Window aggregate with proximity operator (##> - in order)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description @@@ ('laptop' ##> 10 ##> 'professionals')
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description @@@ ('laptop' ##> 10 ##> 'professionals')
ORDER BY rating DESC
LIMIT 3;

-- =============================================================================
-- BASIC TOPN TESTS
-- =============================================================================

-- Test 1: Basic TopN without window aggregates
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    category,
    rating
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    category,
    rating
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 2: TopN with COUNT(*) OVER () - Basic window aggregate
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 3: Multiple window aggregates (COUNT, SUM, AVG)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    price,
    COUNT(*) OVER () as total_count,
    SUM(price) OVER () as total_price,
    AVG(rating) OVER () as avg_rating
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    price,
    COUNT(*) OVER () as total_count,
    SUM(price) OVER () as total_price,
    AVG(rating) OVER () as avg_rating
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 4: Window aggregate with PARTITION BY (not supported yet, but should extract)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    category,
    rating,
    COUNT(*) OVER (PARTITION BY category) as category_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    category,
    rating,
    COUNT(*) OVER (PARTITION BY category) as category_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 5: Window aggregate with ORDER BY in OVER clause
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    price,
    SUM(price) OVER (ORDER BY rating DESC) as running_total
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    price,
    SUM(price) OVER (ORDER BY rating DESC) as running_total
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 6: Window aggregate with ROWS frame
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    AVG(rating) OVER (ORDER BY rating DESC ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) as moving_avg
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    AVG(rating) OVER (ORDER BY rating DESC ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) as moving_avg
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 7: MIN and MAX window aggregates
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    price,
    MIN(price) OVER () as min_price,
    MAX(price) OVER () as max_price
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    price,
    MIN(price) OVER () as min_price,
    MAX(price) OVER () as max_price
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 8: Window aggregate with FILTER clause
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    in_stock,
    COUNT(*) FILTER (WHERE in_stock = true) OVER () as in_stock_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    in_stock,
    COUNT(*) FILTER (WHERE in_stock = true) OVER () as in_stock_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 9: COUNT with specific column (not *)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(brand) OVER () as brand_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(brand) OVER () as brand_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 10: Complex PARTITION BY and ORDER BY combination
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    category,
    brand,
    rating,
    COUNT(*) OVER (PARTITION BY category ORDER BY rating DESC) as category_rank_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    category,
    brand,
    rating,
    COUNT(*) OVER (PARTITION BY category ORDER BY rating DESC) as category_rank_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 11: Window aggregate without ORDER BY on base query
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description @@@ 'laptop';

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description @@@ 'laptop';

-- Test 12: Window aggregate with RANGE frame
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    price,
    SUM(price) OVER (ORDER BY rating RANGE BETWEEN 0.5 PRECEDING AND 0.5 FOLLOWING) as range_sum
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    price,
    SUM(price) OVER (ORDER BY rating RANGE BETWEEN 0.5 PRECEDING AND 0.5 FOLLOWING) as range_sum
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 13: Multiple different PARTITION BY clauses
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    category,
    brand,
    rating,
    COUNT(*) OVER (PARTITION BY category) as by_category,
    COUNT(*) OVER (PARTITION BY brand) as by_brand,
    COUNT(*) OVER () as total
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    category,
    brand,
    rating,
    COUNT(*) OVER (PARTITION BY category) as by_category,
    COUNT(*) OVER (PARTITION BY brand) as by_brand,
    COUNT(*) OVER () as total
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 14: Window aggregate with GROUPS frame (PG17+)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER (ORDER BY rating GROUPS BETWEEN 1 PRECEDING AND CURRENT ROW) as group_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER (ORDER BY rating GROUPS BETWEEN 1 PRECEDING AND CURRENT ROW) as group_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 15: TopN with no @@@ operator (should not trigger window function handling)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE rating > 4.5
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE rating > 4.5
ORDER BY rating DESC
LIMIT 3;

-- Test 16: Window aggregate with multiple base table columns
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    category,
    brand,
    price,
    rating,
    sales,
    COUNT(*) OVER () as total_count,
    SUM(sales) OVER () as total_sales
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 2;

SELECT 
    id,
    name,
    category,
    brand,
    price,
    rating,
    sales,
    COUNT(*) OVER () as total_count,
    SUM(sales) OVER () as total_sales
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 2;

-- Test 17: Window aggregate in a subquery (TopN in outer query)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT * FROM (
    SELECT 
        id,
        name,
        rating,
        price,
        COUNT(*) OVER () as total_count
    FROM products
    WHERE description @@@ 'laptop'
    ORDER BY rating DESC
    LIMIT 5
) sub
ORDER BY price DESC
LIMIT 2;

SELECT * FROM (
    SELECT 
        id,
        name,
        rating,
        price,
        COUNT(*) OVER () as total_count
    FROM products
    WHERE description @@@ 'laptop'
    ORDER BY rating DESC
    LIMIT 5
) sub
ORDER BY price DESC
LIMIT 2;

-- Test 18: Value Facet - Category distribution (like Elasticsearch value facets)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    category,
    rating,
    COUNT(*) OVER () as total_results,
    COUNT(*) OVER (PARTITION BY category) as category_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    category,
    rating,
    COUNT(*) OVER () as total_results,
    COUNT(*) OVER (PARTITION BY category) as category_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 19: Range Facet - Price buckets (like Elasticsearch range facets)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    price,
    CASE 
        WHEN price < 1000 THEN 'Budget'
        WHEN price < 1500 THEN 'Mid-range'
        ELSE 'Premium'
    END as price_bucket,
    COUNT(*) OVER () as total_results,
    COUNT(*) OVER (PARTITION BY CASE 
        WHEN price < 1000 THEN 'Budget'
        WHEN price < 1500 THEN 'Mid-range'
        ELSE 'Premium'
    END) as bucket_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 5;

SELECT 
    id,
    name,
    price,
    CASE 
        WHEN price < 1000 THEN 'Budget'
        WHEN price < 1500 THEN 'Mid-range'
        ELSE 'Premium'
    END as price_bucket,
    COUNT(*) OVER () as total_results,
    COUNT(*) OVER (PARTITION BY CASE 
        WHEN price < 1000 THEN 'Budget'
        WHEN price < 1500 THEN 'Mid-range'
        ELSE 'Premium'
    END) as bucket_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 5;

-- Test 20: Multi-facet - Brand + Price range (combining multiple facets)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    brand,
    price,
    COUNT(*) OVER () as total_results,
    COUNT(*) OVER (PARTITION BY brand) as brand_count,
    AVG(price) OVER (PARTITION BY brand) as avg_brand_price
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    brand,
    price,
    COUNT(*) OVER () as total_results,
    COUNT(*) OVER (PARTITION BY brand) as brand_count,
    AVG(price) OVER (PARTITION BY brand) as avg_brand_price
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 21: Facet with aggregates - MIN/MAX price per category
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    category,
    price,
    COUNT(*) OVER () as total_results,
    MIN(price) OVER (PARTITION BY category) as category_min_price,
    MAX(price) OVER (PARTITION BY category) as category_max_price
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    category,
    price,
    COUNT(*) OVER () as total_results,
    MIN(price) OVER (PARTITION BY category) as category_min_price,
    MAX(price) OVER (PARTITION BY category) as category_max_price
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 22: Boolean facet - In-stock availability
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    in_stock,
    COUNT(*) OVER () as total_results,
    COUNT(*) OVER (PARTITION BY in_stock) as stock_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    in_stock,
    COUNT(*) OVER () as total_results,
    COUNT(*) OVER (PARTITION BY in_stock) as stock_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 23: Popularity facet - Sales volume buckets
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    sales,
    CASE 
        WHEN sales < 100 THEN 'Low'
        WHEN sales < 150 THEN 'Medium'
        ELSE 'High'
    END as sales_volume,
    COUNT(*) OVER () as total_results,
    SUM(sales) OVER () as total_sales
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    sales,
    CASE 
        WHEN sales < 100 THEN 'Low'
        WHEN sales < 150 THEN 'Medium'
        ELSE 'High'
    END as sales_volume,
    COUNT(*) OVER () as total_results,
    SUM(sales) OVER () as total_sales
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 24: Rating histogram - Rating distribution
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    CASE 
        WHEN rating >= 4.7 THEN 'Excellent (4.7+)'
        WHEN rating >= 4.5 THEN 'Very Good (4.5-4.7)'
        WHEN rating >= 4.0 THEN 'Good (4.0-4.5)'
        ELSE 'Fair (<4.0)'
    END as rating_tier,
    COUNT(*) OVER () as total_results,
    COUNT(*) OVER (PARTITION BY CASE 
        WHEN rating >= 4.7 THEN 'Excellent (4.7+)'
        WHEN rating >= 4.5 THEN 'Very Good (4.5-4.7)'
        WHEN rating >= 4.0 THEN 'Good (4.0-4.5)'
        ELSE 'Fair (<4.0)'
    END) as tier_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 5;

SELECT 
    id,
    name,
    rating,
    CASE 
        WHEN rating >= 4.7 THEN 'Excellent (4.7+)'
        WHEN rating >= 4.5 THEN 'Very Good (4.5-4.7)'
        WHEN rating >= 4.0 THEN 'Good (4.0-4.5)'
        ELSE 'Fair (<4.0)'
    END as rating_tier,
    COUNT(*) OVER () as total_results,
    COUNT(*) OVER (PARTITION BY CASE 
        WHEN rating >= 4.7 THEN 'Excellent (4.7+)'
        WHEN rating >= 4.5 THEN 'Very Good (4.5-4.7)'
        WHEN rating >= 4.0 THEN 'Good (4.0-4.5)'
        ELSE 'Fair (<4.0)'
    END) as tier_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 5;

-- Test 25: Complete faceting scenario - Combining all facet types
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    brand,
    price,
    rating,
    in_stock,
    -- Overall metrics
    COUNT(*) OVER () as total_results,
    AVG(price) OVER () as avg_price,
    AVG(rating) OVER () as avg_rating,
    -- Brand facets
    COUNT(*) OVER (PARTITION BY brand) as brand_count,
    -- Price range facets
    COUNT(*) OVER (PARTITION BY CASE 
        WHEN price < 1500 THEN 'Under $1500'
        ELSE '$1500+'
    END) as price_range_count,
    -- Stock facets
    COUNT(*) OVER (PARTITION BY in_stock) as stock_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    brand,
    price,
    rating,
    in_stock,
    -- Overall metrics
    COUNT(*) OVER () as total_results,
    AVG(price) OVER () as avg_price,
    AVG(rating) OVER () as avg_rating,
    -- Brand facets
    COUNT(*) OVER (PARTITION BY brand) as brand_count,
    -- Price range facets
    COUNT(*) OVER (PARTITION BY CASE 
        WHEN price < 1500 THEN 'Under $1500'
        ELSE '$1500+'
    END) as price_range_count,
    -- Stock facets
    COUNT(*) OVER (PARTITION BY in_stock) as stock_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- =============================================================================
-- QUERY CONTEXT FEATURE FLAG TESTS
-- Testing HAVING_SUPPORT, JOIN_SUPPORT, and SUBQUERY_SUPPORT feature flags
-- =============================================================================

-- Test 26: Window function with HAVING clause (should NOT use custom scan - HAVING_SUPPORT=false)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    category,
    AVG(price) as avg_price,
    COUNT(*) OVER() AS total_count
FROM products
WHERE description @@@ 'laptop'
GROUP BY category
HAVING AVG(price) > 1000
ORDER BY avg_price DESC
LIMIT 3;

SELECT 
    category,
    AVG(price) as avg_price,
    COUNT(*) OVER() AS total_count
FROM products
WHERE description @@@ 'laptop'
GROUP BY category
HAVING AVG(price) > 1000
ORDER BY avg_price DESC
LIMIT 3;

-- Test 27: Window function with JOIN (should NOT use custom scan - JOIN_SUPPORT=false)

-- Create a second table for JOIN testing
CREATE TABLE product_categories (
    name TEXT PRIMARY KEY,
    description TEXT,
    priority INTEGER
);

INSERT INTO product_categories VALUES 
('Laptops', 'Portable computing devices', 1);

CREATE INDEX product_categories_idx ON product_categories
USING bm25(name, description, priority)
WITH (key_field='name');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    p.id,
    p.name,
    p.rating,
    pc.description as category_desc,
    COUNT(*) OVER() AS total_count
FROM products p
JOIN product_categories pc ON p.category = pc.name
WHERE p.description @@@ 'laptop'
ORDER BY p.rating DESC
LIMIT 3;

SELECT 
    p.id,
    p.name,
    p.rating,
    pc.description as category_desc,
    COUNT(*) OVER() AS total_count
FROM products p
JOIN product_categories pc ON p.category = pc.name
WHERE p.description @@@ 'laptop'
ORDER BY p.rating DESC
LIMIT 3;

-- Test 28: Window function in subquery
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT *
FROM (
    SELECT 
        id,
        name,
        rating,
        price,
        COUNT(*) OVER() AS total_count
    FROM products
    WHERE description @@@ 'laptop'
    ORDER BY rating DESC
    LIMIT 5
) subq
WHERE total_count > 0
ORDER BY price DESC
LIMIT 3;

SELECT *
FROM (
    SELECT 
        id,
        name,
        rating,
        price,
        COUNT(*) OVER() AS total_count
    FROM products
    WHERE description @@@ 'laptop'
    ORDER BY rating DESC
    LIMIT 5
) subq
WHERE total_count > 0
ORDER BY price DESC
LIMIT 3;

-- Test 29: Nested subqueries with window functions
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT outer_query.*
FROM (
    SELECT 
        inner_query.*,
        ROW_NUMBER() OVER(ORDER BY total_count DESC) as rank
    FROM (
        SELECT 
            id,
            name,
            rating,
            COUNT(*) OVER() AS total_count
        FROM products
        WHERE description @@@ 'laptop'
        ORDER BY rating DESC
        LIMIT 4
    ) inner_query
) outer_query
WHERE rank <= 2;

SELECT outer_query.*
FROM (
    SELECT 
        inner_query.*,
        ROW_NUMBER() OVER(ORDER BY total_count DESC) as rank
    FROM (
        SELECT 
            id,
            name,
            rating,
            COUNT(*) OVER() AS total_count
        FROM products
        WHERE description @@@ 'laptop'
        ORDER BY rating DESC
        LIMIT 4
    ) inner_query
) outer_query
WHERE rank <= 2;

-- Test 30: Window function with HAVING + JOIN
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    p.category,
    pc.description,
    AVG(p.price) as avg_price,
    COUNT(*) OVER() AS total_count
FROM products p
JOIN product_categories pc ON p.category = pc.name
WHERE p.description @@@ 'laptop'
GROUP BY p.category, pc.description
HAVING AVG(p.price) > 1000
ORDER BY avg_price DESC
LIMIT 2;

SELECT 
    p.category,
    pc.description,
    AVG(p.price) as avg_price,
    COUNT(*) OVER() AS total_count
FROM products p
JOIN product_categories pc ON p.category = pc.name
WHERE p.description @@@ 'laptop'
GROUP BY p.category, pc.description
HAVING AVG(p.price) > 1000
ORDER BY avg_price DESC
LIMIT 2;

-- Test 31: Window function with FILTER clause in different contexts

-- Simple case
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    in_stock,
    COUNT(*) FILTER (WHERE rating > 4.5) OVER() AS high_rating_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    in_stock,
    COUNT(*) FILTER (WHERE rating > 4.5) OVER() AS high_rating_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- With HAVING
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    category,
    AVG(rating) as avg_rating,
    COUNT(*) FILTER (WHERE price > 1500) OVER() AS expensive_count
FROM products
WHERE description @@@ 'laptop'
GROUP BY category
HAVING AVG(rating) > 4.0
ORDER BY avg_rating DESC
LIMIT 2;

SELECT 
    category,
    AVG(rating) as avg_rating,
    COUNT(*) FILTER (WHERE price > 1500) OVER() AS expensive_count
FROM products
WHERE description @@@ 'laptop'
GROUP BY category
HAVING AVG(rating) > 4.0
ORDER BY avg_rating DESC
LIMIT 2;

-- Test 32: Mixed supported and unsupported window functions (all-or-nothing test)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    price,
    COUNT(*) OVER() AS total_count,                    -- Supported (COUNT_ANY=true)
    SUM(price) OVER() AS total_price,                  -- Not supported (SUM=false)
    COUNT(brand) OVER() AS brand_count                 -- Not supported (COUNT=false)
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    price,
    COUNT(*) OVER() AS total_count,                    -- Supported (COUNT_ANY=true)
    SUM(price) OVER() AS total_price,                  -- Not supported (SUM=false)
    COUNT(brand) OVER() AS brand_count                 -- Not supported (COUNT=false)
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 33: Only supported window functions (should use custom scan)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    price,
    COUNT(*) OVER() AS total_count1,                   -- Supported (COUNT_ANY=true)
    COUNT(*) OVER() AS total_count2                    -- Supported (COUNT_ANY=true)
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    price,
    COUNT(*) OVER() AS total_count1,
    COUNT(*) OVER() AS total_count2
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 34: Query without ORDER BY and LIMIT (should NOT use custom scan - ONLY_ALLOW_TOP_N=true)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER() AS total_count
FROM products
WHERE description @@@ 'laptop';

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER() AS total_count
FROM products
WHERE description @@@ 'laptop';

-- Test 35: Query with ORDER BY but no LIMIT (should NOT use custom scan - ONLY_ALLOW_TOP_N=true)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER() AS total_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER() AS total_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC;

-- Test 36: Query with LIMIT but no ORDER BY (should NOT use custom scan - ONLY_ALLOW_TOP_N=true)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER() AS total_count
FROM products
WHERE description @@@ 'laptop'
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER() AS total_count
FROM products
WHERE description @@@ 'laptop'
LIMIT 3;

-- Test 37: Window function with COALESCE
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    price,
    COUNT(*) OVER() AS total_count,
    SUM(COALESCE(price, 0.0)) OVER() AS total_price_with_default,
    AVG(COALESCE(rating, 4.0)) OVER() AS avg_rating_with_default
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    price,
    COUNT(*) OVER() AS total_count,
    SUM(COALESCE(price, 0.0)) OVER() AS total_price_with_default,
    AVG(COALESCE(rating, 4.0)) OVER() AS avg_rating_with_default
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 38: Benchmark query - TopN + COUNT(*) OVER ()
-- Verify this produces TopN execution plan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id, name, description, category, brand, COUNT(*) OVER ()
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 10;

SELECT id, name, description, category, brand, COUNT(*) OVER ()
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 10;

-- Test 39: Benchmark query - TopN + pdb.agg terms (faceting)
-- Verify this produces TopN execution plan with custom aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id, name, description, category, brand, pdb.agg('{"terms": {"field": "brand"}}'::jsonb) OVER ()
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 10;

SELECT id, name, description, category, brand, pdb.agg('{"terms": {"field": "brand"}}'::jsonb) OVER ()
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 10;

-- Test 40: Benchmark query - TopN + pdb.agg avg
-- Verify this produces TopN execution plan with custom aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id, name, description, category, brand, pdb.agg('{"avg": {"field": "rating"}}'::jsonb) OVER ()
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 10;

SELECT id, name, description, category, brand, pdb.agg('{"avg": {"field": "rating"}}'::jsonb) OVER ()
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 10;

-- Test 36: TopN with nested aggregations (window function)
-- Verify that nested "aggs" work correctly in TopN/window context
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, name, brand,
       pdb.agg('{"terms": {"field": "brand", "aggs": {"avg_rating": {"avg": {"field": "rating"}}}}}'::jsonb) OVER () AS brand_with_avg_rating
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 5;

SELECT id, name, brand,
       pdb.agg('{"terms": {"field": "brand", "aggs": {"avg_rating": {"avg": {"field": "rating"}}}}}'::jsonb) OVER () AS brand_with_avg_rating
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 5;

-- Cleanup
DROP TABLE product_categories CASCADE;
DROP TABLE products CASCADE;