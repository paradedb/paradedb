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

-- Test 3.2: GROUP BY with grouping column in the middle
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), category, AVG(price) , rating, SUM(price)
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category, rating
ORDER BY category, rating;

SELECT COUNT(*), category, AVG(price) , rating, SUM(price)
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category, rating
ORDER BY category, rating;

-- Test 3.3: GROUP BY with contradictory WHERE clauses
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*), SUM(price), AVG(rating)
FROM products 
WHERE ((NOT (description @@@ 'laptop')) AND (description @@@ 'laptop'))
GROUP BY category;

SELECT category, COUNT(*), SUM(price), AVG(rating)
FROM products 
WHERE ((NOT (description @@@ 'laptop')) AND (description @@@ 'laptop'))
GROUP BY category;

-- Test 3.4: Tautological WHERE clauses with GROUP BY
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
ORDER BY category;

SELECT category, SUM(price) as total_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

-- Test 6.2: ORDER BY multiple columns including aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, rating, COUNT(*) as cnt, AVG(price) as avg_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category, rating
ORDER BY category;

SELECT category, rating, COUNT(*) as cnt, AVG(price) as avg_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category, rating
ORDER BY category;

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
-- =====================================================================
-- SECTION 8: Reproduction case for fruit types error
-- =====================================================================

-- Test case to reproduce the "incompatible fruit types in tree" error
-- This reproduces the specific failure from the generated test suite

DROP TABLE IF EXISTS users CASCADE;
CREATE TABLE users
(
    id    SERIAL8 NOT NULL PRIMARY KEY,
    uuid  UUID NOT NULL,
    name  TEXT,
    color VARCHAR,
    age   INTEGER,
    price NUMERIC(10,2),
    rating INTEGER
);

-- Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idxusers ON users USING bm25 (id, uuid, name, color, age, price, rating)
WITH (
key_field = 'id',
text_fields = '
            {
                "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
                "name": { "tokenizer": { "type": "keyword" }, "fast": true },
                "color": { "tokenizer": { "type": "keyword" }, "fast": true }
            }',
numeric_fields = '
            {
                "age": { "fast": true },
                "price": { "fast": true },
                "rating": { "fast": true }
            }'
);

-- Set seed for deterministic UUID generation (optional, for extra consistency)
SELECT setseed(0.5);

-- Insert test data specifically designed to trigger the "incompatible fruit types" error
-- Strategy: Create data patterns that stress Tantivy's aggregation field type handling
-- Based on the original failing pattern, we need:
-- 1. Complex boolean queries with mixed field types (text + numeric)
-- 2. Multiple aggregation functions on different numeric field types
-- 3. Sufficient data volume to trigger Tantivy's internal aggregation conflicts

-- First, add some regular data
INSERT into users (uuid, name, color, age, price, rating) VALUES
    (gen_random_uuid(), 'bob', 'blue', 20, 99.99, 4),
    (gen_random_uuid(), 'alice', 'blue', 25, 150.50, 5),
    (gen_random_uuid(), 'sally', 'blue', 22, 175.25, 4);

-- Then add many more records with patterns that might trigger the error
-- Use a mix of data that creates aggregation conflicts
INSERT into users (uuid, name, color, age, price, rating)
SELECT
    gen_random_uuid(),
    CASE (i % 7)
        WHEN 0 THEN 'alice'
        WHEN 1 THEN 'bob'
        WHEN 2 THEN 'sally'
        WHEN 3 THEN 'charlie'
        WHEN 4 THEN 'david'
        WHEN 5 THEN 'emma'
        ELSE 'frank'
    END,
    CASE (i % 8)
        WHEN 0 THEN 'blue'
        WHEN 1 THEN 'blue'
        WHEN 2 THEN 'blue'
        WHEN 3 THEN 'red'
        WHEN 4 THEN 'green'
        WHEN 5 THEN 'blue'
        WHEN 6 THEN 'yellow'
        ELSE 'blue'
    END,
    -- Ages that will create complex boolean filtering
    CASE 
        WHEN i % 10 < 3 THEN 15 + (i % 5)  -- Some under 20
        WHEN i % 10 < 7 THEN 20 + (i % 30) -- Many over 20
        ELSE 50 + (i % 40)                 -- Some much older
    END,
    -- Prices with extreme variations to stress aggregation
    CASE 
        WHEN i % 15 = 0 THEN 0.01          -- Very small values
        WHEN i % 13 = 0 THEN 9999.99       -- Very large values
        WHEN i % 11 = 0 THEN 1000000.00    -- Extremely large values
        ELSE (50 + (i * 37) % 500)::numeric(10,2)  -- Regular values
    END,
    -- Ratings that might cause type conflicts with prices
    CASE 
        WHEN i % 17 = 0 THEN 1
        WHEN i % 13 = 0 THEN 5
        ELSE ((i * 7) % 5) + 1
    END
FROM generate_series(1, 150) AS i;

-- Add some edge case records that are more likely to trigger conflicts
INSERT into users (uuid, name, color, age, price, rating) VALUES
    -- Records with NULL-like characteristics that might confuse aggregation
    (gen_random_uuid(), 'edge_case_1', 'blue', 20, 0.00, 1),
    (gen_random_uuid(), 'edge_case_2', 'blue', 21, 999999.99, 5),
    (gen_random_uuid(), 'edge_case_3', 'blue', 19, 0.01, 1),
    -- Records that create many duplicates in grouping
    (gen_random_uuid(), 'duplicate_name', 'blue', 25, 100.00, 3),
    (gen_random_uuid(), 'duplicate_name', 'blue', 26, 200.00, 4),
    (gen_random_uuid(), 'duplicate_name', 'blue', 27, 300.00, 5),
    (gen_random_uuid(), 'duplicate_name', 'blue', 28, 400.00, 1),
    (gen_random_uuid(), 'duplicate_name', 'blue', 29, 500.00, 2);

CREATE INDEX idxusers_uuid ON users (uuid);
CREATE INDEX idxusers_name ON users (name);
CREATE INDEX idxusers_color ON users (color);
CREATE INDEX idxusers_age ON users (age);
CREATE INDEX idxusers_price ON users (price);
CREATE INDEX idxusers_rating ON users (rating);
ANALYZE;

-- Test the specific failing query that caused the "incompatible fruit types" error
-- This query combines text search with NOT operator and multiple aggregations
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT name, SUM(price), MAX(rating), AVG(age) 
FROM users 
WHERE (users.color @@@ 'blue') AND (NOT (users.age < '20')) 
GROUP BY name;

SELECT name, SUM(price), MAX(rating), AVG(age) 
FROM users 
WHERE (users.color @@@ 'blue') AND (NOT (users.age < '20')) 
GROUP BY name;

--- ----

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT name, SUM(price), MAX(rating), AVG(age) 
FROM users 
WHERE (users.color @@@ 'blue')
GROUP BY name;

SELECT name, SUM(price), MAX(rating), AVG(age) 
FROM users 
WHERE (users.color @@@ 'blue')
GROUP BY name;

--- ----

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT name, SUM(price), MAX(rating)
FROM users 
WHERE (users.color @@@ 'blue')
GROUP BY name;

SELECT name, SUM(price), MAX(rating)
FROM users 
WHERE (users.color @@@ 'blue')
GROUP BY name;

-- Additional test cases to explore the fruit types error boundary conditions
-- Test with different combinations that might trigger the same issue

-- Test with different aggregate combinations
SELECT name, COUNT(*), SUM(price), AVG(price), MIN(rating), MAX(rating)
FROM users 
WHERE color @@@ 'blue' AND age >= 20
GROUP BY name;

-- Test with complex WHERE conditions
SELECT color, COUNT(*), AVG(age) 
FROM users 
WHERE (name @@@ 'bob' OR name @@@ 'alice') AND price > 50
GROUP BY color;

DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS type_test CASCADE;
DROP TABLE IF EXISTS groupby_test CASCADE;

RESET paradedb.enable_aggregate_custom_scan;
