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
    price FLOAT,
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
USING paradedb (id, description, rating, (category::pdb.unicode_words('columnar=true')), price);

-- Test 1.1: COUNT(*) aggregate (already supported)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM products WHERE description ||| 'laptop';

SELECT COUNT(*) FROM products WHERE description ||| 'laptop';

-- Test 1.2: SUM aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM(price) FROM products WHERE description ||| 'laptop';

SELECT SUM(price) FROM products WHERE description ||| 'laptop';

-- Test 1.3: AVG aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG(price) FROM products WHERE description ||| 'laptop';

SELECT AVG(price) FROM products WHERE description ||| 'laptop';

-- Test 1.4: MIN aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MIN(price) FROM products WHERE description ||| 'laptop';

SELECT MIN(price) FROM products WHERE description ||| 'laptop';

-- Test 1.5: MAX aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MAX(price) FROM products WHERE description ||| 'laptop';

SELECT MAX(price) FROM products WHERE description ||| 'laptop';

-- Test 1.6: Multiple aggregates in single query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), SUM(price), AVG(price), MIN(price), MAX(price)
FROM products WHERE description ||| 'laptop';

SELECT COUNT(*), SUM(price), AVG(price), MIN(price), MAX(price)
FROM products WHERE description ||| 'laptop';

-- =====================================================================
-- SECTION 2: Edge Cases and Error Conditions
-- =====================================================================

-- Test 2.1: Empty result set behavior
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM products WHERE description ||| 'nonexistent';

SELECT COUNT(*) FROM products WHERE description ||| 'nonexistent';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM(price) FROM products WHERE description ||| 'nonexistent';

SELECT SUM(price) FROM products WHERE description ||| 'nonexistent';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG(price) FROM products WHERE description ||| 'nonexistent';

SELECT AVG(price) FROM products WHERE description ||| 'nonexistent';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MIN(price) FROM products WHERE description ||| 'nonexistent';

SELECT MIN(price) FROM products WHERE description ||| 'nonexistent';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MAX(price) FROM products WHERE description ||| 'nonexistent';

SELECT MAX(price) FROM products WHERE description ||| 'nonexistent';

-- Test 2.2: Contradictory WHERE clauses (impossible conditions)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM products WHERE ((NOT (category ||| 'Electronics')) AND (category ||| 'Electronics'));

SELECT COUNT(*) FROM products WHERE ((NOT (category ||| 'Electronics')) AND (category ||| 'Electronics'));

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM(price) FROM products WHERE ((NOT (category ||| 'Electronics')) AND (category ||| 'Electronics'));

SELECT SUM(price) FROM products WHERE ((NOT (category ||| 'Electronics')) AND (category ||| 'Electronics'));

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG(rating) FROM products WHERE ((NOT (category ||| 'Electronics')) AND (category ||| 'Electronics'));

SELECT AVG(rating) FROM products WHERE ((NOT (category ||| 'Electronics')) AND (category ||| 'Electronics'));

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MIN(price) FROM products WHERE ((NOT (description ||| 'laptop')) AND (description ||| 'laptop'));

SELECT MIN(price) FROM products WHERE ((NOT (description ||| 'laptop')) AND (description ||| 'laptop'));

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MAX(rating) FROM products WHERE ((NOT (category ||| 'Electronics')) AND (category ||| 'Electronics'));

SELECT MAX(rating) FROM products WHERE ((NOT (category ||| 'Electronics')) AND (category ||| 'Electronics'));

-- Test 2.3: Complex contradictory conditions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), SUM(price)
FROM products
WHERE (description ||| 'laptop') AND ((NOT (rating < 4)) AND (rating < 4));

SELECT COUNT(*), SUM(price)
FROM products
WHERE (description ||| 'laptop') AND ((NOT (rating < 4)) AND (rating < 4));

-- =====================================================================
-- SECTION 3: Data Type Compatibility Tests
-- =====================================================================

DROP TABLE IF EXISTS type_test CASCADE;
CREATE TABLE type_test (
    id SERIAL PRIMARY KEY,
    int_val INTEGER,
    bigint_val BIGINT,
    smallint_val SMALLINT,
    numeric_val FLOAT,
    float_val FLOAT,
    double_val DOUBLE PRECISION,
    text_val TEXT
);

INSERT INTO type_test (int_val, bigint_val, smallint_val, numeric_val, float_val, double_val, text_val) VALUES
    (100, 1000000, 10, 99.99, 1.5, 3.14159, 'test1'),
    (200, 2000000, 20, 199.99, 2.5, 6.28318, 'test2'),
    (300, 3000000, 30, 299.99, 3.5, 9.42477, 'test3');

CREATE INDEX type_test_idx ON type_test
USING paradedb (id, (text_val::pdb.unicode_words('columnar=true')), int_val, bigint_val, smallint_val, numeric_val, float_val, double_val);

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
WHERE (text_val ||| 'test1' OR text_val ||| 'test2');

SELECT
    SUM(int_val), AVG(int_val), MIN(int_val), MAX(int_val),
    SUM(bigint_val), AVG(bigint_val), MIN(bigint_val), MAX(bigint_val),
    SUM(smallint_val), AVG(smallint_val), MIN(smallint_val), MAX(smallint_val),
    SUM(numeric_val), AVG(numeric_val), MIN(numeric_val), MAX(numeric_val),
    SUM(float_val), AVG(float_val), MIN(float_val), MAX(float_val),
    SUM(double_val), AVG(double_val), MIN(double_val), MAX(double_val)
FROM type_test
WHERE (text_val ||| 'test1' OR text_val ||| 'test2');

-- =====================================================================
-- SECTION 4: Validation and Fallback Tests
-- =====================================================================

-- Test 4.1: DISTINCT aggregates (should fall back to PostgreSQL)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(DISTINCT category), SUM(price)
FROM products
WHERE description ||| 'laptop';

SELECT COUNT(DISTINCT category), SUM(price)
FROM products
WHERE description ||| 'laptop';

-- Test 4.2: Aggregate on non-fast field (should fall back)
-- Note: description field is not marked as "fast" in the index
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT AVG(LENGTH(description))
FROM products
WHERE category ||| 'Electronics';

SELECT AVG(LENGTH(description))
FROM products
WHERE category ||| 'Electronics';

-- =====================================================================
-- SECTION 6: Complex Query Patterns
-- =====================================================================

-- Test 6.1: Complex boolean WHERE clauses with aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), SUM(price), AVG(rating)
FROM products
WHERE ((description ||| 'laptop') OR (description ||| 'keyboard'))
  AND (rating >= 4 OR category ||| 'Electronics');

SELECT COUNT(*), SUM(price), AVG(rating)
FROM products
WHERE ((description ||| 'laptop') OR (description ||| 'keyboard'))
  AND (rating >= 4 OR category ||| 'Electronics');

-- Test 6.2: Nested boolean expressions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), AVG(price), MIN(rating), MAX(rating)
FROM products
WHERE (NOT (NOT (category ||| 'Electronics'))) AND ((description ||| 'laptop' OR description ||| 'keyboard'));

SELECT COUNT(*), AVG(price), MIN(rating), MAX(rating)
FROM products
WHERE (NOT (NOT (category ||| 'Electronics'))) AND ((description ||| 'laptop' OR description ||| 'keyboard'));

-- =====================================================================
-- SECTION 7: COALESCE Null Values
-- =====================================================================

INSERT INTO products (description, rating, category, price, in_stock) VALUES
    (NULL, NULL, NULL, NULL, NULL);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), AVG(price), MIN(rating), MAX(rating)
FROM products
WHERE id @@@ paradedb.all();

SELECT COUNT(*), AVG(price), MIN(rating), MAX(rating)
FROM products
WHERE id @@@ paradedb.all();

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), AVG(COALESCE(price, 0)), MIN(COALESCE(rating, 0)), MAX(COALESCE(rating, 0))
FROM products
WHERE id @@@ paradedb.all();

SELECT COUNT(*), AVG(COALESCE(price, 0)), MIN(COALESCE(rating, 0)), MAX(COALESCE(rating, 0))
FROM products
WHERE id @@@ paradedb.all();

SELECT COUNT(*), AVG(COALESCE(price, 0)), MIN(COALESCE(rating, 0)), MAX(COALESCE(rating, 0))
FROM products;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), AVG(COALESCE(price, 1)), MIN(COALESCE(rating, 1)), MAX(COALESCE(rating, 1))
FROM products
WHERE id @@@ paradedb.all();

SELECT COUNT(*), AVG(COALESCE(price, 1)), MIN(COALESCE(rating, 1)), MAX(COALESCE(rating, 1))
FROM products
WHERE id @@@ paradedb.all();

SELECT COUNT(*), AVG(COALESCE(price, 1)), MIN(COALESCE(rating, 1)), MAX(COALESCE(rating, 1))
FROM products;

-- =====================================================================
-- SECTION 8: Array Counts
-- =====================================================================
-- COUNT(array) counts non-NULL rows, including empty arrays and arrays whose
-- elements are all NULL. Tantivy's value_count counts stored elements instead.
CREATE TABLE count_array_test (id int PRIMARY KEY, ints int[], words text[]);
INSERT INTO count_array_test VALUES
    (1, '{3,1,1}', '{red,blue,red}'),
    (2, '{}', '{}'),
    (3, NULL, NULL),
    (4, '{NULL,NULL}', '{NULL,NULL}'),
    (5, '{NULL,3}', '{NULL,red}'),
    (6, '{{1,2},{3,4}}', '{{red,blue},{green,red}}');
CREATE INDEX count_array_test_idx ON count_array_test
USING paradedb (id, ints, words) WITH (
    text_fields = '{"words": {"fast": true, "tokenizer": {"type": "raw"}}}'
);
ANALYZE count_array_test;
SET max_parallel_workers_per_gather = 0;
RESET paradedb.max_term_agg_buckets;

-- With the normal bucket cap, array counts must use PostgreSQL aggregation.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, COUNT(ints), COUNT(words) FROM count_array_test
WHERE id @@@ pdb.all() GROUP BY id ORDER BY id;
SELECT id, COUNT(ints), COUNT(words) FROM count_array_test
WHERE id @@@ pdb.all() GROUP BY id ORDER BY id;

SELECT COUNT(*), COUNT(ints), COUNT(words),
       COUNT(ints) FILTER (WHERE id > 2)
FROM count_array_test WHERE id @@@ pdb.all();
SELECT COUNT(words) FROM count_array_test WHERE id @@@ pdb.all();

-- Compare with PostgreSQL without aggregate pushdown.
SET paradedb.enable_aggregate_custom_scan = off;
SELECT id, COUNT(ints), COUNT(words) FROM count_array_test
WHERE id @@@ pdb.all() GROUP BY id ORDER BY id;
SELECT COUNT(*), COUNT(ints), COUNT(words),
       COUNT(ints) FILTER (WHERE id > 2)
FROM count_array_test WHERE id @@@ pdb.all();
SELECT COUNT(words) FROM count_array_test WHERE id @@@ pdb.all();
SET paradedb.enable_aggregate_custom_scan = on;

-- Scalar counts still use the Tantivy aggregate path.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, COUNT(*), COUNT(id) FROM count_array_test
WHERE id @@@ pdb.all() GROUP BY id ORDER BY id;
SELECT id, COUNT(*), COUNT(id) FROM count_array_test
WHERE id @@@ pdb.all() GROUP BY id ORDER BY id;

-- Window counts must also count rows before LIMIT, not stored array elements.
-- A supported COUNT(*) before the array counts must fall back with them.
SELECT id, COUNT(*) OVER (), COUNT(ints) OVER (), COUNT(words) OVER ()
FROM count_array_test WHERE id @@@ pdb.all() ORDER BY id LIMIT 2;
SELECT id, COUNT(words) OVER ()
FROM count_array_test WHERE id @@@ pdb.all() ORDER BY id LIMIT 2;

-- Scalar window counts still use the Tantivy Top-K path.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, COUNT(*) OVER (), COUNT(id) OVER ()
FROM count_array_test WHERE id @@@ pdb.all() ORDER BY id LIMIT 2;
SELECT id, COUNT(*) OVER (), COUNT(id) OVER ()
FROM count_array_test WHERE id @@@ pdb.all() ORDER BY id LIMIT 2;

DROP TABLE count_array_test;
RESET max_parallel_workers_per_gather;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS type_test CASCADE;

RESET paradedb.enable_aggregate_custom_scan;
