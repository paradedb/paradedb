-- Test for the JoinScan Custom Scan planning
-- This test verifies that the join custom scan is proposed when:
-- 1. Query has a LIMIT clause
-- 2. At least one side has a BM25 index with a @@@ predicate

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;

-- Create test tables
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    supplier_id INTEGER,
    price DECIMAL(10,2)
);

CREATE TABLE suppliers (
    id SERIAL PRIMARY KEY,
    name TEXT,
    contact_info TEXT,
    country TEXT
);

-- Insert test data
INSERT INTO suppliers (name, contact_info, country) VALUES
('TechCorp', 'contact@techcorp.com wireless technology', 'USA'),
('GlobalSupply', 'info@globalsupply.com international shipping', 'UK'),
('FastParts', 'sales@fastparts.com quick delivery', 'Germany'),
('QualityFirst', 'quality@first.com premium products', 'Japan');

INSERT INTO products (name, description, supplier_id, price) VALUES
('Wireless Mouse', 'Ergonomic wireless mouse with Bluetooth connectivity', 1, 29.99),
('USB Cable', 'High-speed USB-C cable for fast data transfer', 2, 9.99),
('Keyboard', 'Mechanical keyboard with RGB lighting', 1, 89.99),
('Monitor Stand', 'Adjustable monitor stand for ergonomic setup', 3, 49.99),
('Webcam', 'HD webcam for video conferencing', 4, 59.99),
('Headphones', 'Wireless noise-canceling headphones with premium sound', 1, 199.99),
('Mouse Pad', 'Large gaming mouse pad with wireless charging', 2, 29.99),
('Cable Organizer', 'Desktop cable organizer for clean setup', 3, 14.99);

-- Create BM25 indexes on both tables
CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, description) WITH (key_field = 'id');
CREATE INDEX suppliers_bm25_idx ON suppliers USING bm25 (id, name, contact_info, country) WITH (key_field = 'id');

-- =============================================================================
-- TEST 1: JoinScan should NOT be proposed without LIMIT
-- =============================================================================

-- Make sure the GUC is enabled
SET paradedb.enable_join_custom_scan = on;

-- Query without LIMIT - JoinScan should NOT be proposed
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless';

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id;

-- =============================================================================
-- TEST 2: JoinScan should be proposed with LIMIT and search predicate
-- =============================================================================

-- Query with LIMIT and search predicate on products (which has BM25 index)
-- JoinScan SHOULD be proposed
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
LIMIT 10;

-- =============================================================================
-- TEST 3: JoinScan should be proposed even if search predicate is on only one side
-- =============================================================================

-- Query with LIMIT, predicate only on products
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'keyboard'
ORDER BY p.id
LIMIT 5;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'keyboard'
ORDER BY p.id
LIMIT 5;

-- Query with LIMIT, predicate only on suppliers
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE s.contact_info @@@ 'wireless'
ORDER BY s.id
LIMIT 5;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE s.contact_info @@@ 'wireless'
ORDER BY s.id
LIMIT 5;

-- =============================================================================
-- TEST 4: JoinScan should NOT be proposed for non-INNER joins (for now)
-- =============================================================================

-- LEFT JOIN with LIMIT - should NOT use JoinScan (M1 only handles INNER JOIN)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
LEFT JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
LEFT JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 5: JoinScan can be disabled via GUC
-- =============================================================================

SET paradedb.enable_join_custom_scan = off;

-- Same query as TEST 2, but with GUC disabled - should NOT use JoinScan
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 10;

-- Re-enable for cleanup verification
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 6: ORDER BY pdb.score() - Single Feature join pattern
-- =============================================================================

-- This is the canonical "Single Feature" join pattern from the TopN spec
-- NOTE: Score propagation through JoinScan is not yet implemented in M1.
-- The score() function returns NULL for now. This will be addressed in M2.
SELECT p.id, p.name, s.name AS supplier_name, paradedb.score(p.id)
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY paradedb.score(p.id) DESC
LIMIT 5;

-- =============================================================================
-- TEST 7: Both sides have search predicates
-- =============================================================================

-- When both sides have @@@ predicates with LIMIT, JoinScan should be proposed
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.contact_info @@@ 'technology'
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.contact_info @@@ 'technology'
LIMIT 10;

-- =============================================================================
-- TEST 8: Multi-table joins (3 tables)
-- =============================================================================

-- Create a third table for multi-table join testing
DROP TABLE IF EXISTS categories CASCADE;
CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT
);

INSERT INTO categories (name, description) VALUES
('Electronics', 'Electronic devices and accessories'),
('Office', 'Office supplies and equipment'),
('Gaming', 'Gaming peripherals and accessories');

CREATE INDEX categories_bm25_idx ON categories USING bm25 (id, name, description) WITH (key_field = 'id');

-- Add category_id to products
ALTER TABLE products ADD COLUMN category_id INTEGER;
UPDATE products SET category_id = 1 WHERE id IN (1, 3, 5, 6);
UPDATE products SET category_id = 2 WHERE id IN (2, 4, 8);
UPDATE products SET category_id = 3 WHERE id = 7;

-- 3-table join with LIMIT
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name, c.name AS category_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE p.description @@@ 'wireless'
LIMIT 5;

SELECT p.id, p.name, s.name AS supplier_name, c.name AS category_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 5;

-- =============================================================================
-- TEST 9: Non-equijoin conditions (arbitrary join expressions)
-- =============================================================================

-- Join with non-equality condition
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id >= s.id AND p.supplier_id <= s.id + 1
WHERE p.description @@@ 'wireless'
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id >= s.id AND p.supplier_id <= s.id + 1
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 10: LIMIT without ORDER BY vs with ORDER BY
-- =============================================================================

-- LIMIT without ORDER BY - should still use JoinScan
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'mouse'
LIMIT 3;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'mouse'
LIMIT 3;

-- LIMIT with ORDER BY on non-score column
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'mouse'
ORDER BY p.price DESC
LIMIT 3;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'mouse'
ORDER BY p.price DESC
LIMIT 3;

-- =============================================================================
-- TEST 11: Aggregate Score pattern - OR across tables (without LIMIT)
-- =============================================================================

-- NOTE: This case should propose JoinScan even WITHOUT LIMIT because
-- there's a join-level search predicate (OR spanning both relations).
-- This is the "Aggregate Score" pattern from the spec (planned for M3).
-- Currently falls back to Hash Join since M1 only handles LIMIT cases.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' OR s.contact_info @@@ 'wireless';

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' OR s.contact_info @@@ 'wireless'
ORDER BY p.id;

-- =============================================================================
-- TEST 12: OR across tables WITH LIMIT
-- =============================================================================

-- Same as TEST 11 but with LIMIT.
-- JoinScan is NOT proposed because the OR predicate spans both relations
-- at the join level - neither side has an isolated @@@ predicate.
-- This is the "Aggregate Score" pattern which requires M3's join-level
-- predicate handling.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' OR s.contact_info @@@ 'wireless'
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' OR s.contact_info @@@ 'wireless'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;
DROP TABLE IF EXISTS categories CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
