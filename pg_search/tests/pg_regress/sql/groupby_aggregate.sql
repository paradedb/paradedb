-- Test GROUP BY functionality in aggregate custom scan
-- This file combines and consolidates tests from multiple GROUP BY test files

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- ===========================================================================
-- SECTION 1: Basic GROUP BY Tests with Numeric Fields
-- ===========================================================================

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
    ('Budget laptop for students', 3, 'Electronics', 499.99, false),
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

-- Test 1.1: Basic GROUP BY with integer field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, COUNT(*) AS count
FROM products 
WHERE description @@@ 'laptop' 
GROUP BY rating
ORDER BY rating;

SELECT rating, COUNT(*) AS count
FROM products 
WHERE description @@@ 'laptop' 
GROUP BY rating
ORDER BY rating;

-- Test 1.2: GROUP BY with SUM aggregate
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

-- Test 1.3: GROUP BY with AVG aggregate  
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

-- Test 1.4: GROUP BY with MIN and MAX aggregates
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

-- Test 1.5: GROUP BY with multiple aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) AS count, SUM(price) AS total, AVG(price) AS avg, MIN(price) AS min_price, MAX(price) AS max_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

SELECT category, COUNT(*) AS count, SUM(price) AS total, AVG(price) AS avg, MIN(price) AS min_price, MAX(price) AS max_price
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category
ORDER BY category;

-- Test 1.2: Non-GROUP BY aggregate (should still use custom scan)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) AS total_laptops
FROM products 
WHERE description @@@ 'laptop';

SELECT COUNT(*) AS total_laptops
FROM products 
WHERE description @@@ 'laptop';

-- Test 1.3: GROUP BY with string field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) AS count
FROM products 
WHERE description @@@ 'laptop OR keyboard OR shoes' 
GROUP BY category
ORDER BY category DESC;

SELECT category, COUNT(*) AS count
FROM products 
WHERE description @@@ 'laptop OR keyboard OR shoes' 
GROUP BY category
ORDER BY category DESC;

-- Test 1.4: Test different column orders (COUNT(*) first vs last)
-- Verify that both column orders work correctly
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), category FROM products WHERE description @@@ 'laptop' GROUP BY category;

SELECT COUNT(*), category FROM products WHERE description @@@ 'laptop' GROUP BY category;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) FROM products WHERE description @@@ 'laptop' GROUP BY category;

SELECT category, COUNT(*) FROM products WHERE description @@@ 'laptop' GROUP BY category;

-- Test 1.5: Verify execution plans
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, COUNT(*) FROM products WHERE description @@@ 'laptop' GROUP BY rating;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM products WHERE description @@@ 'laptop';

-- ===========================================================================
-- SECTION 2: Data Type Tests
-- ===========================================================================

DROP TABLE IF EXISTS type_test CASCADE;
CREATE TABLE type_test (
    id SERIAL PRIMARY KEY,
    content TEXT,
    val_int2 SMALLINT,
    val_int4 INTEGER,
    val_int8 BIGINT,
    val_float4 REAL,
    val_float8 DOUBLE PRECISION,
    val_text TEXT,
    val_bool BOOLEAN
);

INSERT INTO type_test (content, val_int2, val_int4, val_int8, val_float4, val_float8, val_text, val_bool) VALUES
    ('alpha test data', 1, 100, 1000000, 1.5, 2.5, 'group_a', true),
    ('alpha test data', 1, 100, 1000000, 1.5, 2.5, 'group_a', true),
    ('beta test data', 2, 200, 2000000, 3.5, 4.5, 'group_b', false),
    ('beta test data', 2, 200, 2000000, 3.5, 4.5, 'group_b', false),
    ('gamma test data', 3, 300, 3000000, 5.5, 6.5, 'group_c', true);

CREATE INDEX type_test_idx ON type_test
USING bm25 (id, content, val_int2, val_int4, val_int8, val_float4, val_float8, val_text, val_bool)
WITH (
    key_field='id',
    text_fields='{"content": {}, "val_text": {"fast": true}}',
    numeric_fields='{
        "val_int2": {"fast": true},
        "val_int4": {"fast": true},
        "val_int8": {"fast": true},
        "val_float4": {"fast": true},
        "val_float8": {"fast": true}
    }',
    boolean_fields='{"val_bool": {"fast": true}}'
);

-- Test 2.1: GROUP BY different numeric types
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT val_int2, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int2 ORDER BY val_int2;

SELECT val_int2, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int2 ORDER BY val_int2;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT val_int4, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int4 ORDER BY val_int4;

SELECT val_int4, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int4 ORDER BY val_int4;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT val_int8, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int8 ORDER BY val_int8;

SELECT val_int8, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int8 ORDER BY val_int8;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT val_float4, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_float4 ORDER BY val_float4;

SELECT val_float4, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_float4 ORDER BY val_float4;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT val_float8, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_float8 ORDER BY val_float8;

SELECT val_float8, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_float8 ORDER BY val_float8;

-- Test 2.2: GROUP BY text field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT val_text, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_text ORDER BY val_text;

SELECT val_text, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_text ORDER BY val_text;

-- Test 2.3: GROUP BY boolean field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT val_bool, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_bool ORDER BY val_bool;

SELECT val_bool, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_bool ORDER BY val_bool;

-- ===========================================================================
-- SECTION 3: Edge Cases and Negative Tests
-- ===========================================================================

-- Test 3.1: GROUP BY with no matching results
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, COUNT(*) AS count
FROM products 
WHERE description @@@ 'nonexistent_term' 
GROUP BY rating;

SELECT rating, COUNT(*) AS count
FROM products 
WHERE description @@@ 'nonexistent_term' 
GROUP BY rating;

-- Test 3.2: Test with non-fast field (should NOT use aggregate scan)
DROP INDEX products_idx;
CREATE INDEX products_idx ON products 
USING bm25 (id, description, rating)
WITH (
    key_field='id',
    text_fields='{"description": {}}',
    numeric_fields='{"rating": {"fast": false}}'  -- Not a fast field
);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, COUNT(*) 
FROM products 
WHERE description @@@ 'laptop' 
GROUP BY rating;

-- Test 3.3: GROUP BY without WHERE clause (should NOT use aggregate scan)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, COUNT(*) 
FROM products 
GROUP BY rating;

-- ===========================================================================
-- SECTION 4: Real-World Example - Support Ticket Analysis
-- ===========================================================================

DROP TABLE IF EXISTS support_tickets CASCADE;
CREATE TABLE support_tickets (
    id SERIAL PRIMARY KEY,
    description TEXT,
    priority TEXT,
    status TEXT,
    category TEXT
);

INSERT INTO support_tickets (description, priority, status, category) VALUES
    ('Cannot login to failed account', 'High', 'Open', 'Authentication'),
    ('Password reset not working (failed)', 'High', 'Open', 'Authentication'),
    ('Slow dashboard loading', 'Medium', 'In Progress', 'Performance'),
    ('Export feature broken error', 'Low', 'Open', 'Features'),
    ('Payment failed error', 'High', 'Resolved', 'Billing'),
    ('Missing invoice', 'Low', 'Resolved', 'Billing');

CREATE INDEX tickets_idx ON support_tickets
USING bm25 (id, description, priority, status, category)
WITH (
    key_field='id',
    text_fields='{
        "description": {},
        "priority": {"fast": true},
        "status": {"fast": true},
        "category": {"fast": true}
    }'
);

-- Test 4.1: Analyze priority distribution for login issues
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT priority, COUNT(*) as count
FROM support_tickets
WHERE description @@@ 'login OR password OR authentication'
GROUP BY priority
ORDER BY priority;

SELECT priority, COUNT(*) as count
FROM support_tickets
WHERE description @@@ 'login OR password OR authentication'
GROUP BY priority
ORDER BY priority;

-- Test 4.2: Status breakdown by category (without ORDER BY)
-- Note: ORDER BY aggregate columns is not yet supported in custom scan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) as count
FROM support_tickets
WHERE description @@@ 'error OR broken OR failed'
GROUP BY category;

SELECT category, COUNT(*) as count
FROM support_tickets
WHERE description @@@ 'error OR broken OR failed'
GROUP BY category;

-- ===========================================================================
-- SECTION 5: Multi-Column GROUP BY
-- ===========================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, priority, COUNT(*) 
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category, priority;

-- ---------------------------------------------------------------------------
-- Test 5.1: Multi-column GROUP BY with NO aggregate function (2 columns)
-- ---------------------------------------------------------------------------
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, priority
FROM support_tickets
WHERE description @@@ 'error'
GROUP BY category, priority
ORDER BY category, priority;

SELECT category, priority
FROM support_tickets
WHERE description @@@ 'error'
GROUP BY category, priority
ORDER BY category, priority;

-- ---------------------------------------------------------------------------
-- Test 5.2: Three-column GROUP BY with COUNT(*)
-- ---------------------------------------------------------------------------
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, priority, status, COUNT(*)
FROM support_tickets
WHERE description @@@ 'error'
GROUP BY category, priority, status
ORDER BY priority, status;

SELECT category, priority, status, COUNT(*)
FROM support_tickets
WHERE description @@@ 'error'
GROUP BY category, priority, status
ORDER BY priority, status;

-- ---------------------------------------------------------------------------
-- Test 5.3: Three-column GROUP BY without aggregates, descending ORDER BY
-- ---------------------------------------------------------------------------
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, priority, status
FROM support_tickets
WHERE description @@@ 'error'
GROUP BY category, priority, status
ORDER BY status DESC, priority DESC;

SELECT category, priority, status
FROM support_tickets
WHERE description @@@ 'error'
GROUP BY category, priority, status
ORDER BY priority DESC, status DESC;

-- ===========================================================================
-- SECTION 6: Verify ORDER BY functionality
-- ===========================================================================
-- Note: Our custom aggregate scan supports ORDER BY on grouping columns,
-- but ORDER BY on aggregate columns falls back to PostgreSQL.

-- Test 6.1: ORDER BY COUNT(*) should fall back to PostgreSQL
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category
ORDER BY COUNT(*) DESC;

-- The query should work with PostgreSQL's standard execution
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category
ORDER BY COUNT(*) DESC;

-- Test 6.2: ORDER BY alias should also fall back to PostgreSQL
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category
ORDER BY count DESC;

-- The query should work with PostgreSQL's standard execution
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category
ORDER BY count DESC;

-- Test 6.3: ORDER BY grouping column should use custom aggregate scan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category
ORDER BY category;

-- This should use our custom aggregate scan with ORDER BY
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category
ORDER BY category;

-- Test 6.4: Verify GROUP BY without ORDER BY still uses our custom aggregate scan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category;

-- This uses our custom aggregate scan
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category;

-- Test 6.5: GROUP BY without aggregates (distinct categories) should use custom aggregate scan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category
FROM support_tickets
WHERE description @@@ 'error'
GROUP BY category
ORDER BY category;

-- This should use our custom aggregate scan and return distinct categories without COUNT(*)
SELECT category
FROM support_tickets
WHERE description @@@ 'error'
GROUP BY category
ORDER BY category;

-- ===========================================================================
-- SECTION 7: Benchmark-style comparison – GROUP BY vs paradedb.aggregate
-- ===========================================================================

-- Test 7.1: GROUP BY with integer field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'failed' 
GROUP BY category
ORDER BY category;

SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'failed' 
GROUP BY category
ORDER BY category;

-- Aggregate UDF equivalent
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT *
FROM paradedb.aggregate(
        index => 'tickets_idx',
        query => paradedb.term('description','failed'),
        agg   => '{"buckets": {"terms": {"field": "category"}}}',
        solve_mvcc => true
);

SELECT *
FROM paradedb.aggregate(
        index => 'tickets_idx',
        query => paradedb.term('description','failed'),
        agg   => '{"buckets": {"terms": {"field": "category"}}}',
        solve_mvcc => true
);
-- ===========================================================================
-- SECTION 7: Benchmark-style comparison – GROUP BY vs paradedb.aggregate
-- ===========================================================================

-- Test 7.1: GROUP BY with integer field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'failed' 
GROUP BY category
ORDER BY category;

SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'failed' 
GROUP BY category
ORDER BY category;

-- Aggregate UDF equivalent
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT *
FROM paradedb.aggregate(
        index => 'tickets_idx',
        query => paradedb.term('description','failed'),
        agg   => '{"buckets": {"terms": {"field": "category"}}}',
        solve_mvcc => true
);

SELECT *
FROM paradedb.aggregate(
        index => 'tickets_idx',
        query => paradedb.term('description','failed'),
        agg   => '{"buckets": {"terms": {"field": "category"}}}',
        solve_mvcc => true
);
-- ===========================================================================
-- SECTION 8: Forced Aggregate Custom Scan Tests (to expose bugs)
-- ===========================================================================

-- Test that forces aggregate custom scan to be used for edge cases
DROP TABLE IF EXISTS min_max_test CASCADE;
CREATE TABLE min_max_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    rating INTEGER
);

INSERT INTO min_max_test (name, rating) VALUES 
    ('bob', 2),
    ('bob', 4), 
    ('bob', 5),
    ('alice', 1),
    ('alice', 3),
    ('alice', 7);

CREATE INDEX min_max_test_idx ON min_max_test USING bm25 (id, name, rating)
WITH (
    key_field = 'id',
    text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}',
    numeric_fields = '{"rating": {"fast": true}}'
);

-- Force aggregate custom scan to be used by disabling parallel processing
-- This test exposes the MIN bug when GROUP BY is on the same field as the aggregate
SET max_parallel_workers_per_gather = 0;
SET enable_hashagg = off;
SET enable_sort = off;

-- Test 8.1: Basic GROUP BY test (fundamental test - should return 3 rows for bob: ratings 2, 4, 5)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating FROM min_max_test WHERE name @@@ 'bob' GROUP BY rating ORDER BY rating;

-- This should return exactly 3 rows: rating values 2, 4, 5
-- BUG: Currently returns individual rows instead of grouped results
SELECT rating FROM min_max_test WHERE name @@@ 'bob' GROUP BY rating ORDER BY rating;

-- Test 8.2: GROUP BY on different field (should return 2 rows: 'alice', 'bob')
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT name FROM min_max_test GROUP BY name ORDER BY name;

-- This should return exactly 2 rows: 'alice', 'bob'
-- BUG: Currently returns individual rows instead of grouped results
SELECT name FROM min_max_test GROUP BY name ORDER BY name;

-- Test 8.3: GROUP BY with COUNT (should work correctly if GROUP BY is fixed)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT name, COUNT(*) FROM min_max_test GROUP BY name ORDER BY name;

-- This should return: ('alice', 3), ('bob', 3)
SELECT name, COUNT(*) FROM min_max_test GROUP BY name ORDER BY name;

-- Test 8.4: MIN with GROUP BY on same field (known to be buggy)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, MIN(rating) FROM min_max_test WHERE name @@@ 'bob' GROUP BY rating ORDER BY rating;

SELECT rating, MIN(rating) FROM min_max_test WHERE name @@@ 'bob' GROUP BY rating ORDER BY rating;

-- Test 8.5: MAX with GROUP BY on same field 
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, MAX(rating) FROM min_max_test WHERE name @@@ 'bob' GROUP BY rating ORDER BY rating;

SELECT rating, MAX(rating) FROM min_max_test WHERE name @@@ 'bob' GROUP BY rating ORDER BY rating;

-- Test 8.6: SUM with GROUP BY on same field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, SUM(rating) FROM min_max_test WHERE name @@@ 'bob' GROUP BY rating ORDER BY rating;

SELECT rating, SUM(rating) FROM min_max_test WHERE name @@@ 'bob' GROUP BY rating ORDER BY rating;

-- Test 8.7: AVG with GROUP BY (should return NULL due to current bug)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT name, AVG(rating) FROM min_max_test GROUP BY name ORDER BY name;

-- This should return: ('alice', 3.67), ('bob', 3.67) 
-- BUG: Currently returns NULL for AVG
SELECT name, AVG(rating) FROM min_max_test GROUP BY name ORDER BY name;

-- Test 8.8: Complex boolean query like proptest - this should expose the real bug
-- This mirrors the failing proptest: OR(NOT(name='bob'), name='bob') which is always true
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating FROM min_max_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY rating ORDER BY rating;

-- This should return exactly 4 rows: 1, 2, 3, 4, 5, 7 (all unique ratings)
-- BUG: Currently returns individual rows instead of grouped results
SELECT rating FROM min_max_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY rating ORDER BY rating;

-- Test 8.9: Another complex boolean query like proptest
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT name FROM min_max_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY name ORDER BY name;

-- This should return exactly 2 rows: 'alice', 'bob'
-- BUG: Currently returns individual rows instead of grouped results  
SELECT name FROM min_max_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY name ORDER BY name;

-- Test 8.10: Complex boolean with aggregates (like proptest failure)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, AVG(rating), name FROM min_max_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY name, rating ORDER BY name, rating;

-- This should return grouped results, not individual rows
SELECT rating, AVG(rating), name FROM min_max_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY name, rating ORDER BY name, rating;


-- ===========================================================================
-- SECTION 9: Core GROUP BY Bug Tests (Reproduces proptest failures)
-- ===========================================================================

-- Test to expose the fundamental GROUP BY bug that causes proptest failures
-- The issue: aggregate custom scan returns individual rows instead of grouped results

DROP TABLE IF EXISTS groupby_bug_test CASCADE;
CREATE TABLE groupby_bug_test (
    id SERIAL PRIMARY KEY,
    age INTEGER,
    name TEXT
);

INSERT INTO groupby_bug_test (age, name) VALUES 
    (25, 'alice'),
    (25, 'bob'),   -- Two people age 25 - should group to 1 row
    (30, 'carol'),
    (30, 'dave'),  -- Two people age 30 - should group to 1 row  
    (35, 'eve');   -- One person age 35 - should be 1 row

CREATE INDEX groupby_bug_idx ON groupby_bug_test 
USING bm25 (id, age, name)
WITH (
    key_field='id',
    text_fields='{"name": {"fast": true}}',
    numeric_fields='{"age": {"fast": true}}'
);

-- Force aggregate custom scan
SET max_parallel_workers_per_gather = 0;
SET enable_hashagg = off;
SET enable_sort = off;

-- Test 9.1: Basic GROUP BY test 
-- CRITICAL BUG: Should return 3 rows (25, 30, 35) but returns 5 individual rows
-- This is the core of the proptest failure
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT age FROM groupby_bug_test GROUP BY age ORDER BY age;

-- Expected: 3 rows (25, 30, 35)
-- BUG: Returns 5 rows (25, 25, 30, 30, 35) - individual rows, not grouped!
SELECT age FROM groupby_bug_test GROUP BY age ORDER BY age;

-- Test 9.2: The exact failing proptest pattern
-- OR(NOT(name='bob'), name='bob') = TRUE for all rows, should behave like Test 9.1
-- This is the simplified version of the failing proptest query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT age FROM groupby_bug_test 
WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') 
GROUP BY age ORDER BY age;

-- Expected: 3 rows (25, 30, 35) - same as Test 9.1
-- BUG: Returns 5 individual rows without proper grouping
SELECT age FROM groupby_bug_test 
WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') 
GROUP BY age ORDER BY age;

-- Test 9.3: GROUP BY with COUNT should also fail
-- Expected: 3 rows with counts (25|2, 30|2, 35|1)  
-- BUG: Returns 5 rows with count=1 for each (no grouping performed)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT age, COUNT(*) FROM groupby_bug_test GROUP BY age ORDER BY age;

SELECT age, COUNT(*) FROM groupby_bug_test GROUP BY age ORDER BY age;



-- ===========================================================================
-- SECTION 9: Core GROUP BY Bug Tests (Reproduces proptest failures)
-- ===========================================================================

-- Test to expose the fundamental GROUP BY bug that causes proptest failures
-- The issue: aggregate custom scan returns individual rows instead of grouped results
DROP TABLE IF EXISTS groupby_bug_test CASCADE;
CREATE TABLE groupby_bug_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    age INTEGER
);

INSERT INTO groupby_bug_test (name, age) VALUES 
    ('alice', 23), ('bob', 33), ('alice', 23), ('charlie', 53), 
    ('bob', 68), ('alice', 71), ('charlie', 74), ('alice', 8),
    ('bob', 85), ('charlie', 87), ('alice', 96), ('bob', 23),
    ('alice', 33), ('charlie', 53), ('bob', 68), ('alice', 71);

CREATE INDEX groupby_bug_idx ON groupby_bug_test 
USING bm25 (id, name, age)
WITH (
    key_field='id',
    text_fields='{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}',
    numeric_fields='{"age": {"fast": true}}'
);

-- Force aggregate custom scan settings
SET max_parallel_workers_per_gather = 0;
SET enable_hashagg = off;
SET enable_sort = off;

-- Test 9.1: This should return 10 unique ages, but aggregate custom scan returns 16 individual rows
-- Query: (NOT (name = 'bob')) OR (name = 'bob') is a tautology (always true)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT age FROM groupby_bug_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY age ORDER BY age;

-- BUG: This returns 16 rows instead of 10 grouped rows
-- Expected: 10 unique age values (23, 33, 53, 68, 71, 74, 8, 85, 87, 96)
-- Actual: 16 individual age values (one for each row in the table)
SELECT age FROM groupby_bug_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY age ORDER BY age;

-- Test 9.2: Verify the correct behavior with regular PostgreSQL (for comparison)
SET paradedb.enable_aggregate_custom_scan = off;
SELECT age FROM groupby_bug_test WHERE (NOT (name = 'bob')) OR (name = 'bob') GROUP BY age ORDER BY age;
SET paradedb.enable_aggregate_custom_scan = on;

-- Test 9.3: Same bug exists with multiple GROUP BY columns  
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT age, name FROM groupby_bug_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY age, name ORDER BY age, name;

-- BUG: This also returns 16 rows instead of grouped results
SELECT age, name FROM groupby_bug_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY age, name ORDER BY age, name;

-- ===========================================================================
-- SECTION 10: Exact Proptest Failure Reproduction
-- ===========================================================================

-- This is the EXACT query that fails in the proptest
-- Expected: 10 grouped age values like ["2", "24", "37", "39", "61", "7", "71", "72", "8", "90"]
-- Actual: 40+ individual age values like ["1", "11", "17", "2", "20", "24", "25", ...]

-- This shows the fundamental bug: aggregate custom scan returns individual document values
-- instead of performing GROUP BY aggregation

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT age FROM groupby_bug_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY age ORDER BY age;

-- CRITICAL BUG: This returns individual rows (one per document) instead of grouped values
-- This is the core of all proptest failures - GROUP BY is completely broken
SELECT age FROM groupby_bug_test WHERE (NOT (name @@@ 'bob')) OR (name @@@ 'bob') GROUP BY age ORDER BY age;


-- Regression test for ALL aggregate NULL handling on impossible WHERE clauses
-- This should return NULL instead of panicking with "result should be a number/float"

-- Recreate the full products index with category field for the final tests
DROP INDEX products_idx;
CREATE INDEX products_idx ON products 
USING bm25 (id, description, rating, category, price)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"rating": {"fast": true}, "price": {"fast": true}}'
);

SELECT 'Testing MAX with impossible WHERE clause (should return NULL)' as test_case;
SELECT MAX(rating) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

SELECT 'Testing MIN with impossible WHERE clause (should return NULL)' as test_case;  
SELECT MIN(price) FROM products WHERE ((NOT (description @@@ 'laptop')) AND (description @@@ 'laptop'));

SELECT 'Testing AVG with impossible WHERE clause (should return NULL)' as test_case;
SELECT AVG(rating) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

SELECT 'Testing SUM with impossible WHERE clause (should return NULL)' as test_case;
SELECT SUM(price) FROM products WHERE ((NOT (description @@@ 'laptop')) AND (description @@@ 'laptop'));

SELECT 'Testing COUNT with impossible WHERE clause (should return NULL)' as test_case;
SELECT COUNT(*) FROM products WHERE ((NOT (category @@@ 'Electronics')) AND (category @@@ 'Electronics'));

-- More complex contradictory cases
SELECT 'Testing MAX with complex contradiction (should return NULL)' as test_case;
SELECT MAX(rating) FROM products WHERE (((category @@@ 'Electronics') AND (NOT (category @@@ 'Electronics'))) AND (description @@@ 'laptop'));

SELECT 'Testing MIN with complex contradiction (should return NULL)' as test_case;
SELECT MIN(price) FROM products WHERE (((category @@@ 'Sports') AND (NOT (description @@@ 'laptop'))) AND (description @@@ 'laptop'));

-- ===========================================================================
-- Clean up
-- ===========================================================================

-- Reset settings
RESET max_parallel_workers_per_gather;
RESET enable_hashagg;
RESET enable_sort;
RESET paradedb.enable_aggregate_custom_scan;

DROP TABLE support_tickets CASCADE;
DROP TABLE type_test CASCADE;
DROP TABLE products CASCADE;
DROP TABLE min_max_test CASCADE;
DROP TABLE groupby_bug_test CASCADE; 
