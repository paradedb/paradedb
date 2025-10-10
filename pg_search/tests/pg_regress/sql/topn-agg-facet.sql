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

-- Test 1: Basic TopN without window aggregates
\echo 'Test 1: Basic TopN query'
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
\echo 'Test 2: TopN with COUNT(*) OVER () (basic window aggregate)'
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
\echo 'Test 3: Multiple window aggregates in one query'
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
\echo 'Test 4: COUNT with PARTITION BY category'
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

-- Test 5: Window aggregate with ORDER BY in OVER clause
\echo 'Test 5: SUM with ORDER BY in OVER clause (running total)'
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

-- Test 6: Window aggregate with ROWS frame
\echo 'Test 6: AVG with ROWS frame'
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

-- Test 7: MIN and MAX window aggregates
\echo 'Test 7: MIN and MAX aggregates'
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
\echo 'Test 8: COUNT with FILTER clause'
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

-- Test 9: COUNT with specific column (not *)
\echo 'Test 9: COUNT(column) instead of COUNT(*)'
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

-- Test 10: Complex PARTITION BY and ORDER BY combination
\echo 'Test 10: Complex window with PARTITION BY and ORDER BY'
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

-- Test 11: Window aggregate without ORDER BY on base query
\echo 'Test 11: Window aggregate without ORDER BY or LIMIT'
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
\echo 'Test 12: SUM with RANGE frame'
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

-- Test 13: Multiple different PARTITION BY clauses
\echo 'Test 13: Multiple window functions with different partitions'
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

-- Test 14: Window aggregate with GROUPS frame (PG17+)
\echo 'Test 14: COUNT with GROUPS frame'
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

-- Test 15: TopN with no @@@ operator (should not trigger window function handling)
\echo 'Test 15: Window aggregate without @@@ operator (should use standard WindowAgg)'
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
\echo 'Test 16: Window aggregate with all result columns'
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
\echo 'Test 17: Subquery with window aggregate, outer LIMIT'
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
\echo 'Test 18: TopN with category facet (value facet pattern)'
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
\echo 'Test 19: TopN with price range buckets'
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
\echo 'Test 20: TopN with multiple facets (brand + price range)'
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
\echo 'Test 21: TopN with MIN/MAX aggregates per category'
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
\echo 'Test 22: TopN with boolean facet (in_stock)'
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
\echo 'Test 23: TopN with sales volume facets'
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
\echo 'Test 24: TopN with rating distribution histogram'
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
\echo 'Test 25: Complete e-commerce faceting (brand, price, rating, stock)'
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

-- Cleanup
DROP TABLE products CASCADE;