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
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT rating, COUNT(*) AS count
FROM products 
WHERE description @@@ 'laptop' 
GROUP BY rating;
-- ORDER BY rating;

SELECT rating, COUNT(*) AS count
FROM products 
WHERE description @@@ 'laptop' 
GROUP BY rating;
-- ORDER BY rating;

-- Test 1.2: Non-GROUP BY aggregate (should still use custom scan)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS total_laptops
FROM products 
WHERE description @@@ 'laptop';

SELECT COUNT(*) AS total_laptops
FROM products 
WHERE description @@@ 'laptop';

-- Test 1.3: GROUP BY with string field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT category, COUNT(*) AS count
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category;
-- ORDER BY category;

SELECT category, COUNT(*) AS count
FROM products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category;
-- ORDER BY category;

-- Test 1.4: Test different column orders (COUNT(*) first vs last)
-- Verify that both column orders work correctly
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*), category FROM products WHERE description @@@ 'laptop' GROUP BY category;

SELECT COUNT(*), category FROM products WHERE description @@@ 'laptop' GROUP BY category;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT category, COUNT(*) FROM products WHERE description @@@ 'laptop' GROUP BY category;

SELECT category, COUNT(*) FROM products WHERE description @@@ 'laptop' GROUP BY category;

-- Test 1.5: Verify execution plans
EXPLAIN (COSTS OFF, VERBOSE)
SELECT rating, COUNT(*) FROM products WHERE description @@@ 'laptop' GROUP BY rating;

EXPLAIN (COSTS OFF, VERBOSE)
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
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT val_int2, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int2; -- ORDER BY val_int2;

SELECT val_int2, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int2; -- ORDER BY val_int2;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT val_int4, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int4; -- ORDER BY val_int4;

SELECT val_int4, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int4; -- ORDER BY val_int4;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT val_int8, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int8; -- ORDER BY val_int8;

SELECT val_int8, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_int8; -- ORDER BY val_int8;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT val_float4, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_float4; -- ORDER BY val_float4;

SELECT val_float4, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_float4; -- ORDER BY val_float4;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT val_float8, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_float8; -- ORDER BY val_float8;

SELECT val_float8, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_float8; -- ORDER BY val_float8;

-- Test 2.2: GROUP BY text field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT val_text, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_text; -- ORDER BY val_text;

SELECT val_text, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_text; -- ORDER BY val_text;

-- Test 2.3: GROUP BY boolean field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT val_bool, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_bool; -- ORDER BY val_bool;

SELECT val_bool, COUNT(*) FROM type_test WHERE content @@@ 'test' GROUP BY val_bool; -- ORDER BY val_bool;

-- ===========================================================================
-- SECTION 3: Edge Cases and Negative Tests
-- ===========================================================================

-- Test 3.1: GROUP BY with no matching results
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
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

EXPLAIN (COSTS OFF, VERBOSE) 
SELECT rating, COUNT(*) 
FROM products 
WHERE description @@@ 'laptop' 
GROUP BY rating;

-- Test 3.3: GROUP BY without WHERE clause (should NOT use aggregate scan)
EXPLAIN (COSTS OFF, VERBOSE) 
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
    ('Cannot login to account', 'High', 'Open', 'Authentication'),
    ('Password reset not working', 'High', 'Open', 'Authentication'),
    ('Slow dashboard loading', 'Medium', 'In Progress', 'Performance'),
    ('Export feature broken', 'Low', 'Open', 'Features'),
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
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT priority, COUNT(*) as count
FROM support_tickets
WHERE description @@@ 'login OR password OR authentication'
GROUP BY priority;
-- ORDER BY priority;

SELECT priority, COUNT(*) as count
FROM support_tickets
WHERE description @@@ 'login OR password OR authentication'
GROUP BY priority;
-- ORDER BY priority;

-- Test 4.2: Status breakdown by category (without ORDER BY)
-- Note: ORDER BY aggregate columns is not yet supported in custom scan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT category, COUNT(*) as count
FROM support_tickets
WHERE description @@@ 'error OR broken OR failed'
GROUP BY category;

SELECT category, COUNT(*) as count
FROM support_tickets
WHERE description @@@ 'error OR broken OR failed'
GROUP BY category;

-- ===========================================================================
-- SECTION 5: Multi-Column GROUP BY (Falls back to PostgreSQL)
-- ===========================================================================

-- This will fall back to PostgreSQL's standard GroupAggregate as we don't support multi-column GROUP BY yet
EXPLAIN (COSTS OFF, VERBOSE) 
SELECT category, priority, COUNT(*) 
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category, priority;

-- ===========================================================================
-- SECTION 6: Verify ORDER BY functionality
-- ===========================================================================
-- Note: Our custom aggregate scan supports ORDER BY on grouping columns,
-- but ORDER BY on aggregate columns falls back to PostgreSQL.

-- Test 6.1: ORDER BY COUNT(*) should fall back to PostgreSQL
EXPLAIN (COSTS OFF, VERBOSE) 
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
EXPLAIN (COSTS OFF, VERBOSE) 
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
EXPLAIN (COSTS OFF, VERBOSE) 
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
EXPLAIN (COSTS OFF, VERBOSE) 
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category;

-- This uses our custom aggregate scan
SELECT category, COUNT(*) as count
FROM support_tickets 
WHERE description @@@ 'error' 
GROUP BY category;

-- ===========================================================================
-- Clean up
-- ===========================================================================

DROP TABLE support_tickets CASCADE;
DROP TABLE type_test CASCADE;
DROP TABLE products CASCADE; 
