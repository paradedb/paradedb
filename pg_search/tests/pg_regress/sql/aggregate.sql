-- =====================================================================
-- Aggregate Custom Scan (Non-GROUP BY) Functionality
-- =====================================================================
-- This file tests aggregate functions (COUNT, SUM, AVG, MIN, MAX) 
-- without GROUP BY clauses for the aggregate custom scan feature.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- SECTION 1: Basic Aggregate Function Tests
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

-- Test 1.1: COUNT(*) aggregate (already supported)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM products WHERE description @@@ 'laptop';

SELECT COUNT(*) FROM products WHERE description @@@ 'laptop';

-- Test 1.2: SUM aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM(price) FROM products WHERE description @@@ 'laptop';

SELECT SUM(price) FROM products WHERE description @@@ 'laptop';

-- Test 1.3: AVG aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG(price) FROM products WHERE description @@@ 'laptop';

SELECT AVG(price) FROM products WHERE description @@@ 'laptop';

-- Test 1.4: MIN aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MIN(price) FROM products WHERE description @@@ 'laptop';

SELECT MIN(price) FROM products WHERE description @@@ 'laptop';

-- Test 1.5: MAX aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MAX(price) FROM products WHERE description @@@ 'laptop';

SELECT MAX(price) FROM products WHERE description @@@ 'laptop';

-- Test 1.6: Multiple aggregates in single query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), SUM(price), AVG(price), MIN(price), MAX(price) 
FROM products WHERE description @@@ 'laptop';

SELECT COUNT(*), SUM(price), AVG(price), MIN(price), MAX(price) 
FROM products WHERE description @@@ 'laptop';

-- =====================================================================
-- SECTION 2: Edge Cases and Error Conditions
-- =====================================================================

-- Test 2.1: Empty result set behavior
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM products WHERE description @@@ 'nonexistent';

SELECT COUNT(*) FROM products WHERE description @@@ 'nonexistent';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM(price) FROM products WHERE description @@@ 'nonexistent';

SELECT SUM(price) FROM products WHERE description @@@ 'nonexistent';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG(price) FROM products WHERE description @@@ 'nonexistent';

SELECT AVG(price) FROM products WHERE description @@@ 'nonexistent';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MIN(price) FROM products WHERE description @@@ 'nonexistent';

SELECT MIN(price) FROM products WHERE description @@@ 'nonexistent';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MAX(price) FROM products WHERE description @@@ 'nonexistent';

SELECT MAX(price) FROM products WHERE description @@@ 'nonexistent';

-- Test 2.2: Contradictory WHERE clauses (impossible conditions)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

SELECT COUNT(*) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM(price) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

SELECT SUM(price) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG(rating) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

SELECT AVG(rating) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MIN(price) FROM products WHERE ((NOT (description @@@ 'laptop')) AND (description @@@ 'laptop'));

SELECT MIN(price) FROM products WHERE ((NOT (description @@@ 'laptop')) AND (description @@@ 'laptop'));

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MAX(rating) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

SELECT MAX(rating) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

-- Test 2.3: Complex contradictory conditions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), SUM(price) 
FROM products 
WHERE (description @@@ 'laptop') AND ((NOT (rating < 4)) AND (rating < 4));

SELECT COUNT(*), SUM(price) 
FROM products 
WHERE (description @@@ 'laptop') AND ((NOT (rating < 4)) AND (rating < 4));

-- =====================================================================
-- SECTION 3: Data Type Compatibility Tests
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

-- Test 3.1: Aggregates on different numeric types
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT 
    SUM(int_val), AVG(int_val), MIN(int_val), MAX(int_val),
    SUM(bigint_val), AVG(bigint_val), MIN(bigint_val), MAX(bigint_val),
    SUM(smallint_val), AVG(smallint_val), MIN(smallint_val), MAX(smallint_val),
    SUM(numeric_val), AVG(numeric_val), MIN(numeric_val), MAX(numeric_val),
    SUM(float_val), AVG(float_val), MIN(float_val), MAX(float_val),
    SUM(double_val), AVG(double_val), MIN(double_val), MAX(double_val)
FROM type_test 
WHERE text_val @@@ 'test1 OR test2';

SELECT 
    SUM(int_val), AVG(int_val), MIN(int_val), MAX(int_val),
    SUM(bigint_val), AVG(bigint_val), MIN(bigint_val), MAX(bigint_val),
    SUM(smallint_val), AVG(smallint_val), MIN(smallint_val), MAX(smallint_val),
    SUM(numeric_val), AVG(numeric_val), MIN(numeric_val), MAX(numeric_val),
    SUM(float_val), AVG(float_val), MIN(float_val), MAX(float_val),
    SUM(double_val), AVG(double_val), MIN(double_val), MAX(double_val)
FROM type_test 
WHERE text_val @@@ 'test1 OR test2';

-- =====================================================================
-- SECTION 4: Validation and Fallback Tests
-- =====================================================================

-- Test 4.1: DISTINCT aggregates (should fall back to PostgreSQL)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(DISTINCT category), SUM(price)
FROM products 
WHERE description @@@ 'laptop';

SELECT COUNT(DISTINCT category), SUM(price)
FROM products 
WHERE description @@@ 'laptop';

-- Test 4.2: Aggregate on non-fast field (should fall back)
-- Note: description field is not marked as "fast" in the index
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG(LENGTH(description))
FROM products 
WHERE category @@@ 'Electronics';

SELECT AVG(LENGTH(description))
FROM products 
WHERE category @@@ 'Electronics';

-- =====================================================================
-- SECTION 6: Complex Query Patterns
-- =====================================================================

-- Test 6.1: Complex boolean WHERE clauses with aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), SUM(price), AVG(rating)
FROM products 
WHERE ((description @@@ 'laptop') OR (description @@@ 'keyboard')) 
  AND (rating >= 4 OR category @@@ 'Electronics');

SELECT COUNT(*), SUM(price), AVG(rating)
FROM products 
WHERE ((description @@@ 'laptop') OR (description @@@ 'keyboard')) 
  AND (rating >= 4 OR category @@@ 'Electronics');

-- Test 6.2: Nested boolean expressions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), AVG(price), MIN(rating), MAX(rating)
FROM products 
WHERE (NOT (NOT (category @@@ 'Electronics'))) AND (description @@@ 'laptop OR keyboard');

SELECT COUNT(*), AVG(price), MIN(rating), MAX(rating)
FROM products 
WHERE (NOT (NOT (category @@@ 'Electronics'))) AND (description @@@ 'laptop OR keyboard');

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS type_test CASCADE;

RESET paradedb.enable_aggregate_custom_scan;
