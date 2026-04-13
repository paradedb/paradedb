-- Test for MPP (plan partitioning) JoinScan execution
-- Verifies that the enable_mpp_join GUC exists and is settable,
-- and that queries produce correct results when MPP is enabled.

-- Disable parallel workers initially for setup
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP: Create tables with enough data for multi-segment indexes
-- =============================================================================

DROP TABLE IF EXISTS mpp_orders CASCADE;
DROP TABLE IF EXISTS mpp_customers CASCADE;

CREATE TABLE mpp_customers (
    id INTEGER PRIMARY KEY,
    name TEXT,
    city TEXT,
    age INTEGER
);

CREATE TABLE mpp_orders (
    id INTEGER PRIMARY KEY,
    customer_id INTEGER,
    product TEXT,
    amount DECIMAL(10,2)
);

-- Create BM25 indexes BEFORE inserting to encourage multiple segments
CREATE INDEX mpp_customers_bm25 ON mpp_customers USING bm25 (id, name, city, age)
WITH (key_field = 'id', text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true}, "city": {"tokenizer": {"type": "keyword"}, "fast": true}}', numeric_fields = '{"age": {"fast": true}}');

CREATE INDEX mpp_orders_bm25 ON mpp_orders USING bm25 (id, customer_id, product, amount)
WITH (key_field = 'id', text_fields = '{"product": {"tokenizer": {"type": "keyword"}, "fast": true}}', numeric_fields = '{"customer_id": {"fast": true}, "amount": {"fast": true}}');

-- Insert customer data
INSERT INTO mpp_customers (id, name, city, age) VALUES
(1, 'alice', 'NYC', 30),
(2, 'bob', 'LA', 25),
(3, 'cloe', 'NYC', 35),
(4, 'dave', 'SF', 28),
(5, 'eve', 'LA', 32);

-- Insert order data
INSERT INTO mpp_orders (id, customer_id, product, amount) VALUES
(101, 1, 'laptop', 999.99),
(102, 1, 'mouse', 29.99),
(103, 2, 'keyboard', 89.99),
(104, 3, 'monitor', 499.99),
(105, 3, 'laptop', 1299.99),
(106, 4, 'mouse', 19.99),
(107, 5, 'keyboard', 79.99),
(108, 5, 'laptop', 899.99);

-- Create standard indexes for join keys
CREATE INDEX mpp_orders_customer_id ON mpp_orders (customer_id);

-- =============================================================================
-- TEST 1: Verify the GUC exists and is settable
-- =============================================================================

-- Should not error
SET paradedb.enable_mpp_join = off;
SHOW paradedb.enable_mpp_join;

SET paradedb.enable_mpp_join = on;
SHOW paradedb.enable_mpp_join;

-- Reset for baseline
SET paradedb.enable_mpp_join = off;

-- =============================================================================
-- TEST 2: Baseline query results with broadcast-join (MPP off)
-- =============================================================================

SET paradedb.enable_join_custom_scan = on;
SET max_parallel_workers_per_gather = 2;
SET max_parallel_workers = 4;

-- Inner join with search predicate
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name, o.product, o.amount
FROM mpp_customers c
JOIN mpp_orders o ON c.id = o.customer_id
WHERE c.name @@@ 'alice'
ORDER BY c.id, o.id
LIMIT 10;

SELECT c.id, c.name, o.product, o.amount
FROM mpp_customers c
JOIN mpp_orders o ON c.id = o.customer_id
WHERE c.name @@@ 'alice'
ORDER BY c.id, o.id
LIMIT 10;

-- =============================================================================
-- TEST 3: Same query with MPP enabled
-- =============================================================================

SET paradedb.enable_mpp_join = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name, o.product, o.amount
FROM mpp_customers c
JOIN mpp_orders o ON c.id = o.customer_id
WHERE c.name @@@ 'alice'
ORDER BY c.id, o.id
LIMIT 10;

SELECT c.id, c.name, o.product, o.amount
FROM mpp_customers c
JOIN mpp_orders o ON c.id = o.customer_id
WHERE c.name @@@ 'alice'
ORDER BY c.id, o.id
LIMIT 10;

-- =============================================================================
-- TEST 4: Multi-predicate query
-- =============================================================================

-- MPP still on
SELECT c.id, c.name, c.city, o.product, o.amount
FROM mpp_customers c
JOIN mpp_orders o ON c.id = o.customer_id
WHERE c.city @@@ 'NYC' AND o.product @@@ 'laptop'
ORDER BY c.id, o.id
LIMIT 10;

-- Compare with MPP off
SET paradedb.enable_mpp_join = off;

SELECT c.id, c.name, c.city, o.product, o.amount
FROM mpp_customers c
JOIN mpp_orders o ON c.id = o.customer_id
WHERE c.city @@@ 'NYC' AND o.product @@@ 'laptop'
ORDER BY c.id, o.id
LIMIT 10;

-- =============================================================================
-- CLEANUP
-- =============================================================================

SET paradedb.enable_mpp_join = off;
SET max_parallel_workers_per_gather = 0;
DROP TABLE IF EXISTS mpp_orders CASCADE;
DROP TABLE IF EXISTS mpp_customers CASCADE;
