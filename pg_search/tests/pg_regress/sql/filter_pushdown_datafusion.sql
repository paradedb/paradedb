-- Test for filter pushdown through DataFusion
-- This test verifies that DataFusion's supports_filters_pushdown is called
-- for cross-table OR conditions with @@@ predicates.
--
-- Key pattern: (cond on tbl1) OR (cond on tbl2)
-- PostgreSQL cannot push this down because it's an OR across two tables.
-- JoinScan creates SearchPredicateUDF markers, then DataFusion's optimizer
-- may try to push them down to individual table scans.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;

-- Create test tables
CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    supplier_id INTEGER,
    price NUMERIC(10,2),
    stock INTEGER
);

CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    country TEXT,
    rating INTEGER
);

-- Insert test data for products
INSERT INTO products (id, name, description, supplier_id, price, stock) VALUES
(1, 'Laptop', 'High performance laptop computer', 1, 999.99, 10),
(2, 'Mouse', 'Wireless ergonomic mouse', 1, 49.99, 100),
(3, 'Keyboard', 'Mechanical gaming keyboard', 1, 129.99, 50),
(4, 'Monitor', 'Ultra-wide curved monitor', 2, 599.99, 25),
(5, 'Webcam', 'HD webcam for video calls', 2, 79.99, 75),
(6, 'Headphones', 'Noise canceling headphones', 3, 299.99, 30),
(7, 'Microphone', 'USB condenser microphone', 3, 149.99, 40),
(8, 'Speaker', 'Bluetooth portable speaker', 4, 89.99, 60),
(9, 'Tablet', 'Android tablet device', 4, 449.99, 20),
(10, 'Charger', 'Fast charging USB-C charger', 5, 29.99, 200);

-- Insert test data for suppliers
INSERT INTO suppliers (id, name, description, country, rating) VALUES
(1, 'TechCorp', 'Leading technology manufacturer', 'USA', 5),
(2, 'DisplayPro', 'Premium display solutions', 'Japan', 4),
(3, 'AudioMax', 'Professional audio equipment', 'Germany', 5),
(4, 'GadgetWorld', 'Consumer electronics retailer', 'China', 3),
(5, 'PowerTech', 'Power and charging solutions', 'USA', 4);

-- Create BM25 indexes
CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, description, supplier_id, price, stock)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}, "price": {"fast": true}, "stock": {"fast": true}}');

CREATE INDEX suppliers_bm25_idx ON suppliers USING bm25 (id, name, description, country, rating)
WITH (key_field = 'id', numeric_fields = '{"rating": {"fast": true}}');

-- Enable JoinScan
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- =============================================================================
-- CROSS-TABLE OR TESTS
-- These are the ONLY tests that trigger supports_filters_pushdown in DataFusion
-- because PostgreSQL cannot push down OR conditions across two tables.
-- =============================================================================
-- =============================================================================

-- =============================================================================
-- TEST 1: Simple cross-table OR
-- (p.description @@@ 'X') OR (s.description @@@ 'Y')
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'laptop' OR s.description @@@ 'technology')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'laptop' OR s.description @@@ 'technology')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 2: Cross-table OR with different search terms
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'monitor OR webcam' OR s.description @@@ 'display OR premium')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'monitor OR webcam' OR s.description @@@ 'display OR premium')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 3: Cross-table OR combined with single-table AND
-- The single-table AND part goes to Tantivy Query,
-- the cross-table OR creates SearchPredicateUDF for DataFusion.
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, p.price, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.price > 100
  AND (p.description @@@ 'laptop OR keyboard' OR s.description @@@ 'technology')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, p.price, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.price > 100
  AND (p.description @@@ 'laptop OR keyboard' OR s.description @@@ 'technology')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 4: Multiple predicates in OR across tables
-- (p_cond1 OR p_cond2 OR s_cond1)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'laptop' OR p.description @@@ 'monitor' OR s.description @@@ 'professional')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'laptop' OR p.description @@@ 'monitor' OR s.description @@@ 'professional')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 5: Cross-table OR with range filters
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, p.price, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.stock >= 25
  AND (p.description @@@ 'laptop OR monitor OR headphones' OR s.description @@@ 'audio OR display')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, p.price, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.stock >= 25
  AND (p.description @@@ 'laptop OR monitor OR headphones' OR s.description @@@ 'audio OR display')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 6: Nested cross-table boolean
-- (p_cond1 OR p_cond2) AND (p_cond3 OR s_cond1)
-- The second part is cross-table and creates SearchPredicateUDF.
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'laptop' OR p.description @@@ 'keyboard')
  AND (p.description @@@ 'computer' OR s.description @@@ 'technology')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'laptop' OR p.description @@@ 'keyboard')
  AND (p.description @@@ 'computer' OR s.description @@@ 'technology')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 7: Deeply nested with cross-table at leaf
-- (p_cond1 OR (p_cond2 OR (s_cond1 AND NOT p_cond3)))
-- The s_cond1 makes this cross-table.
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'keyboard' OR (p.description @@@ 'headphones' OR (s.description @@@ 'professional' AND NOT p.description @@@ 'wireless'))
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'keyboard' OR (p.description @@@ 'headphones' OR (s.description @@@ 'professional' AND NOT p.description @@@ 'wireless'))
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 8: Cross-table AND (both tables required)
-- (p_cond1) AND (s_cond1) - both must match
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'laptop OR headphones')
  AND (s.description @@@ 'technology OR audio')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'laptop OR headphones')
  AND (s.description @@@ 'technology OR audio')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 9: Complex nested cross-table
-- ((p_cond1 AND s_cond1) OR (p_cond2 AND s_cond2))
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE ((p.description @@@ 'laptop' AND s.description @@@ 'technology')
    OR (p.description @@@ 'headphones' AND s.description @@@ 'audio'))
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE ((p.description @@@ 'laptop' AND s.description @@@ 'technology')
    OR (p.description @@@ 'headphones' AND s.description @@@ 'audio'))
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 10: NOT with cross-table
-- NOT (p_cond1) OR s_cond1
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (NOT p.description @@@ 'wireless' OR s.description @@@ 'technology')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (NOT p.description @@@ 'wireless' OR s.description @@@ 'technology')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- CLEANUP
-- =============================================================================
DROP TABLE products CASCADE;
DROP TABLE suppliers CASCADE;
