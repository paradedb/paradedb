-- =====================================================================
-- Test Suite for Aggregate Custom Scan GROUP BY Functionality
-- =====================================================================
-- This file tests GROUP BY queries with aggregate functions (COUNT, SUM, AVG, MIN, MAX)
-- for the aggregate custom scan feature.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test Data Setup
-- =====================================================================

DROP TABLE IF EXISTS products CASCADE;
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    rating INTEGER,
    category TEXT,
    price NUMERIC,
    in_stock BOOLEAN
);

INSERT INTO products (description, rating, category, price, in_stock) VALUES
    ('Laptop with fast processor', 5, 'Electronics', 999.99, true),
    ('Gaming laptop with RGB', 5, 'Electronics', 1299.99, true),
    ('Toy laptop for kids', 3, 'Toys', 499.99, false),
    ('Wireless keyboard and mouse', 4, 'Electronics', 79.99, true),
    ('Mechanical keyboard RGB', 5, 'Electronics', 149.99, true),
    ('Running shoes for athletes', 5, 'Sports', 89.99, true),
    ('Winter jacket warm', 4, 'Clothing', 129.99, true),
    ('Summer jacket light', 3, 'Clothing', 59.99, true);

CREATE INDEX products_idx ON products 
USING bm25 (id, description, rating, category, price)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"rating": {"fast": true}, "price": {"fast": true}}'
);

-- =====================================================================
-- SECTION 1: GROUP BY with Aggregate Functions
-- =====================================================================

-- Test 1.1: GROUP BY with COUNT(*)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) 
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

SELECT category, COUNT(*) 
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

-- Test 1.2: GROUP BY with SUM
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, SUM(price) AS total_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

SELECT category, SUM(price) AS total_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

-- Test 1.3: GROUP BY with AVG
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, AVG(price) AS avg_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

SELECT category, AVG(price) AS avg_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

-- Test 1.4: GROUP BY with MIN and MAX
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, MIN(price) AS min_price, MAX(price) AS max_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

SELECT category, MIN(price) AS min_price, MAX(price) AS max_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

-- Test 1.5: GROUP BY with all aggregate functions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, 
       COUNT(*) AS count, 
       SUM(price) AS total, 
       AVG(price) AS avg, 
       MIN(price) AS min_price, 
       MAX(price) AS max_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

SELECT category, 
       COUNT(*) AS count, 
       SUM(price) AS total, 
       AVG(price) AS avg, 
       MIN(price) AS min_price, 
       MAX(price) AS max_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

-- Test 1.6: GROUP BY with numeric field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, COUNT(*), SUM(price), AVG(price)
FROM products 
WHERE description @@@ 'laptop' 
GROUP BY rating
ORDER BY rating;

SELECT rating, COUNT(*), SUM(price), AVG(price)
FROM products 
WHERE description @@@ 'laptop' 
GROUP BY rating
ORDER BY rating;

-- =====================================================================
-- SECTION 2: Multiple GROUP BY Columns
-- =====================================================================

-- Test 2.1: Two GROUP BY columns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, rating, COUNT(*), AVG(price)
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category, rating
ORDER BY category, rating;

SELECT category, rating, COUNT(*), AVG(price)
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category, rating
ORDER BY category, rating;

-- Test 2.2: GROUP BY with different column orders
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), category FROM products WHERE description @@@ 'laptop' GROUP BY category ORDER BY category;

SELECT COUNT(*), category FROM products WHERE description @@@ 'laptop' GROUP BY category ORDER BY category;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) FROM products WHERE description @@@ 'laptop' GROUP BY category ORDER BY category;

SELECT category, COUNT(*) FROM products WHERE description @@@ 'laptop' GROUP BY category ORDER BY category;

-- =====================================================================
-- SECTION 3: GROUP BY Edge Cases and Error Conditions
-- =====================================================================

-- Test 3.1: GROUP BY with empty result set
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*), SUM(price), AVG(price)
FROM products 
WHERE description @@@ 'nonexistent'
GROUP BY category;

SELECT category, COUNT(*), SUM(price), AVG(price)
FROM products 
WHERE description @@@ 'nonexistent'
GROUP BY category;

-- Test 3.2: GROUP BY with contradictory WHERE clauses
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*), SUM(price), AVG(rating)
FROM products 
WHERE ((NOT (description @@@ 'laptop')) AND (description @@@ 'laptop'))
GROUP BY category;

SELECT category, COUNT(*), SUM(price), AVG(rating)
FROM products 
WHERE ((NOT (description @@@ 'laptop')) AND (description @@@ 'laptop'))
GROUP BY category;

-- Test 3.3: Tautological WHERE clauses with GROUP BY
-- WHERE (NOT (description @@@ 'laptop')) OR (description @@@ 'laptop') is always true
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*), SUM(price), AVG(rating)
FROM products 
WHERE ((NOT (description @@@ 'laptop')) OR (description @@@ 'laptop'))
GROUP BY category;

SELECT category, COUNT(*), SUM(price), AVG(rating)
FROM products 
WHERE ((NOT (description @@@ 'laptop')) OR (description @@@ 'laptop'))
GROUP BY category;

-- =====================================================================
-- SECTION 4: GROUP BY with Different Data Types
-- =====================================================================

DROP TABLE IF EXISTS type_test CASCADE;
CREATE TABLE type_test (
    id SERIAL PRIMARY KEY,
    int_val INTEGER,
    bigint_val BIGINT,
    smallint_val SMALLINT,
    numeric_val NUMERIC(10,2),
    float_val FLOAT,
    double_val DOUBLE PRECISION,
    text_val TEXT
);

INSERT INTO type_test (int_val, bigint_val, smallint_val, numeric_val, float_val, double_val, text_val) VALUES
    (100, 1000000, 10, 99.99, 1.5, 3.14159, 'test1'),
    (200, 2000000, 20, 199.99, 2.5, 6.28318, 'test2'),
    (300, 3000000, 30, 299.99, 3.5, 9.42477, 'test3');

CREATE INDEX type_test_idx ON type_test 
USING bm25 (id, text_val, int_val, bigint_val, smallint_val, numeric_val, float_val, double_val)
WITH (
    key_field='id',
    text_fields='{"text_val": {"fast": true}}',
    numeric_fields='{
        "int_val": {"fast": true},
        "bigint_val": {"fast": true},
        "smallint_val": {"fast": true},
        "numeric_val": {"fast": true},
        "float_val": {"fast": true},
        "double_val": {"fast": true}
    }'
);

-- Test 4.1: GROUP BY with different numeric types
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT text_val, SUM(int_val), AVG(numeric_val), MIN(float_val), MAX(bigint_val)
FROM type_test
WHERE text_val @@@ 'test1 OR test2 OR test3'
GROUP BY text_val
ORDER BY text_val;

SELECT text_val, SUM(int_val), AVG(numeric_val), MIN(float_val), MAX(bigint_val)
FROM type_test
WHERE text_val @@@ 'test1 OR test2 OR test3'
GROUP BY text_val
ORDER BY text_val;

-- =====================================================================
-- SECTION 5: GROUP BY Validation and Fallback Tests
-- =====================================================================

-- Test 5.1: GROUP BY with DISTINCT aggregates (should fall back to PostgreSQL)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(DISTINCT rating), SUM(price)
FROM products 
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category;

SELECT category, COUNT(DISTINCT rating), SUM(price)
FROM products 
WHERE description @@@ 'laptop OR keyboard'
GROUP BY category
ORDER BY category;

-- Test 5.2: GROUP BY field conflicts with aggregate field
-- (same field in both GROUP BY and aggregate - should fall back)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, SUM(price), MAX(rating)
FROM products
WHERE description @@@ 'keyboard'
GROUP BY rating;

SELECT rating, SUM(price), MAX(rating) 
FROM products 
WHERE description @@@ 'keyboard' 
GROUP BY rating
ORDER BY rating;

-- Test 5.3: GROUP BY field conflicts with WHERE search field
-- (category is both searched and grouped - should fall back)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, MIN(rating), MAX(rating), SUM(price)
FROM products 
WHERE category @@@ 'Electronics'
GROUP BY category;

SELECT category, MIN(rating), MAX(rating), SUM(price)
FROM products 
WHERE category @@@ 'Electronics'
GROUP BY category
ORDER BY category;

-- =====================================================================
-- SECTION 6: ORDER BY with GROUP BY Aggregates
-- =====================================================================

-- Test 6.1: ORDER BY aggregate result
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, SUM(price) as total_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY total_price DESC;

SELECT category, SUM(price) as total_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY total_price DESC;

-- Test 6.2: ORDER BY multiple columns including aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, rating, COUNT(*) as cnt, AVG(price) as avg_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category, rating
ORDER BY category, avg_price DESC;

SELECT category, rating, COUNT(*) as cnt, AVG(price) as avg_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category, rating
ORDER BY category, avg_price DESC;

-- =====================================================================
-- SECTION 7: Complex GROUP BY Query Patterns
-- =====================================================================

-- Test 7.1: Complex boolean WHERE clauses with GROUP BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, SUM(price), COUNT(*)
FROM products 
WHERE ((description @@@ 'laptop') OR (description @@@ 'keyboard')) 
  AND (rating >= 4 OR category @@@ 'Electronics')
GROUP BY rating
ORDER BY rating;

SELECT rating, SUM(price), COUNT(*)
FROM products 
WHERE ((description @@@ 'laptop') OR (description @@@ 'keyboard')) 
  AND (rating >= 4 OR category @@@ 'Electronics')
GROUP BY rating
ORDER BY rating;

-- Test 7.2: Nested boolean expressions with GROUP BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, AVG(price), MIN(rating), MAX(rating)
FROM products 
WHERE (NOT (NOT (category @@@ 'Electronics'))) AND (description @@@ 'laptop OR keyboard')
GROUP BY category
ORDER BY category;

SELECT category, AVG(price), MIN(rating), MAX(rating)
FROM products 
WHERE (NOT (NOT (category @@@ 'Electronics'))) AND (description @@@ 'laptop OR keyboard')
GROUP BY category
ORDER BY category;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS type_test CASCADE;
DROP TABLE IF EXISTS groupby_test CASCADE;

RESET paradedb.enable_aggregate_custom_scan;
