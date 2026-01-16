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
-- Using explicit IDs in distinct ranges to differentiate from ctids:
-- Suppliers: IDs 151-154
-- Products: IDs 201-208
CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    supplier_id INTEGER,
    price DECIMAL(10,2)
);

CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY,
    name TEXT,
    contact_info TEXT,
    country TEXT
);

-- Insert test data with explicit IDs
INSERT INTO suppliers (id, name, contact_info, country) VALUES
(151, 'TechCorp', 'contact@techcorp.com wireless technology', 'USA'),
(152, 'GlobalSupply', 'info@globalsupply.com international shipping', 'UK'),
(153, 'FastParts', 'sales@fastparts.com quick delivery', 'Germany'),
(154, 'QualityFirst', 'quality@first.com premium products', 'Japan');

INSERT INTO products (id, name, description, supplier_id, price) VALUES
(201, 'Wireless Mouse', 'Ergonomic wireless mouse with Bluetooth connectivity', 151, 29.99),
(202, 'USB Cable', 'High-speed USB-C cable for fast data transfer', 152, 9.99),
(203, 'Keyboard', 'Mechanical keyboard with RGB lighting', 151, 89.99),
(204, 'Monitor Stand', 'Adjustable monitor stand for ergonomic setup', 153, 49.99),
(205, 'Webcam', 'HD webcam for video conferencing', 154, 59.99),
(206, 'Headphones', 'Wireless noise-canceling headphones with premium sound', 151, 199.99),
(207, 'Mouse Pad', 'Large gaming mouse pad with wireless charging', 152, 29.99),
(208, 'Cable Organizer', 'Desktop cable organizer for clean setup', 153, 14.99);

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

-- -- This is the canonical "Single Feature" join pattern from the TopN spec
-- -- NOTE: Score propagation through JoinScan is not yet implemented in M1.
-- -- The score() function returns NULL for now. This will be addressed in M2.
-- EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
-- SELECT p.id, p.name, s.name AS supplier_name, paradedb.score(p.id)
-- FROM products p
-- JOIN suppliers s ON p.supplier_id = s.id
-- WHERE p.description @@@ 'wireless'
-- ORDER BY paradedb.score(p.id) DESC
-- LIMIT 5;

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
-- TEST 8: Aggregate Score pattern - OR across tables (without LIMIT)
-- =============================================================================

-- NOTE: This case should propose JoinScan even WITHOUT LIMIT because
-- there's a join-level search predicate (OR spanning both relations).
-- This is the "Aggregate Score" pattern from the spec (planned for M3).
-- Currently falls back to Hash Join since M1 only handles LIMIT cases.
-- EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
-- SELECT p.id, p.name, s.name AS supplier_name
-- FROM products p
-- JOIN suppliers s ON p.supplier_id = s.id
-- WHERE p.description @@@ 'wireless' OR s.contact_info @@@ 'wireless';

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' OR s.contact_info @@@ 'wireless'
ORDER BY p.id;

-- =============================================================================
-- TEST 9: OR across tables WITH LIMIT
-- =============================================================================

-- Same as TEST 8 but with LIMIT.
-- JoinScan IS proposed for join-level predicates (OR across tables).
-- The OR condition means a row passes if EITHER the product description
-- contains 'wireless' OR the supplier contact_info contains 'wireless'.
-- EXPECTED: 4 rows matching the OR condition (same as TEST 8).
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
-- TEST 10: Multi-table joins (3 tables) - includes UPDATE that moves product ctids
-- =============================================================================

-- Create a third table for multi-table join testing
-- Categories: IDs 301-303
DROP TABLE IF EXISTS categories CASCADE;
CREATE TABLE categories (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT
);

INSERT INTO categories (id, name, description) VALUES
(301, 'Electronics', 'Electronic devices and accessories'),
(302, 'Office', 'Office supplies and equipment'),
(303, 'Gaming', 'Gaming peripherals and accessories');

CREATE INDEX categories_bm25_idx ON categories USING bm25 (id, name, description) WITH (key_field = 'id');

-- Add category_id to products
ALTER TABLE products ADD COLUMN category_id INTEGER;
UPDATE products SET category_id = 301 WHERE id IN (201, 203, 205, 206);
UPDATE products SET category_id = 302 WHERE id IN (202, 204, 208);
UPDATE products SET category_id = 303 WHERE id = 207;

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
-- TEST 11: OR across tables (without LIMIT) - AFTER UPDATE moved product ctids
-- =============================================================================
-- NOTE: Products were just UPDATED above (category_id added), so their ctids
-- have moved from (0,1)-(0,8) to new locations (0,9)-(0,16).
-- The BM25 index still has the OLD ctids.
-- This test checks if JoinScan handles stale ctids correctly.

-- Debug: Show current product ctids after UPDATE
SELECT 'Products CTIDs after UPDATE (moved from original locations):' AS info;
SELECT ctid, id, name FROM products ORDER BY id;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' OR s.contact_info @@@ 'wireless'
ORDER BY p.id;

-- =============================================================================
-- TEST 12: OR across tables WITH LIMIT - AFTER UPDATE moved product ctids
-- =============================================================================
-- Same as TEST 9 but with LIMIT - uses JoinScan.
-- JoinScan's ctid-based matching for join-level predicates may fail here
-- because the indexed ctids don't match the current heap ctids.
-- EXPECTED: 4 rows (products 201, 203, 206, 207 match 'wireless' in description,
--           plus any products joined to suppliers with 'wireless' in contact_info)

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
-- TEST 13: Non-equijoin conditions (arbitrary join expressions)
-- =============================================================================

-- -- Join with non-equality condition
-- EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
-- SELECT p.id, p.name, s.name AS supplier_name
-- FROM products p
-- JOIN suppliers s ON p.supplier_id >= s.id AND p.supplier_id <= s.id + 1
-- WHERE p.description @@@ 'wireless'
-- LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id >= s.id AND p.supplier_id <= s.id + 1
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 14: LIMIT without ORDER BY vs with ORDER BY
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
-- TEST 15: TEXT join keys (non-integer)
-- =============================================================================

-- Create tables with TEXT join keys
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS customers CASCADE;

CREATE TABLE customers (
    customer_code TEXT PRIMARY KEY,
    name TEXT,
    email TEXT
);

CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    customer_code TEXT,
    description TEXT,
    amount DECIMAL(10,2)
);

INSERT INTO customers (customer_code, name, email) VALUES
('CUST-001', 'Alice Corp', 'alice@corp.com'),
('CUST-002', 'Bob Industries', 'bob@industries.com'),
('CUST-003', 'Carol Enterprises', 'carol@enterprises.com');

INSERT INTO orders (id, customer_code, description, amount) VALUES
(1, 'CUST-001', 'wireless mouse order', 29.99),
(2, 'CUST-001', 'keyboard order premium', 89.99),
(3, 'CUST-002', 'wireless headphones bulk', 599.97),
(4, 'CUST-003', 'monitor stand', 49.99),
(5, 'CUST-002', 'cable wireless charger', 19.99);

CREATE INDEX orders_bm25_idx ON orders USING bm25 (id, description) WITH (key_field = 'id');

-- TEXT join key test
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT o.id, o.description, c.name AS customer_name
FROM orders o
JOIN customers c ON o.customer_code = c.customer_code
WHERE o.description @@@ 'wireless'
LIMIT 10;

SELECT o.id, o.description, c.name AS customer_name
FROM orders o
JOIN customers c ON o.customer_code = c.customer_code
WHERE o.description @@@ 'wireless'
ORDER BY o.id
LIMIT 10;

-- =============================================================================
-- TEST 16: Composite join keys (multiple columns)
-- =============================================================================

-- Create tables with composite keys
DROP TABLE IF EXISTS inventory CASCADE;
DROP TABLE IF EXISTS warehouses CASCADE;

CREATE TABLE warehouses (
    region_id INTEGER,
    warehouse_code TEXT,
    name TEXT,
    description TEXT,
    PRIMARY KEY (region_id, warehouse_code)
);

CREATE TABLE inventory (
    id INTEGER PRIMARY KEY,
    region_id INTEGER,
    warehouse_code TEXT,
    product_name TEXT,
    quantity INTEGER
);

INSERT INTO warehouses (region_id, warehouse_code, name, description) VALUES
(1, 'WH-A', 'East Coast Main', 'Primary warehouse for east coast distribution'),
(1, 'WH-B', 'East Coast Backup', 'Backup warehouse for east coast'),
(2, 'WH-A', 'West Coast Main', 'Primary warehouse for west coast distribution'),
(2, 'WH-B', 'West Coast Express', 'Express shipping warehouse west coast');

INSERT INTO inventory (id, region_id, warehouse_code, product_name, quantity) VALUES
(1, 1, 'WH-A', 'wireless mouse', 100),
(2, 1, 'WH-A', 'keyboard', 50),
(3, 1, 'WH-B', 'monitor', 25),
(4, 2, 'WH-A', 'wireless headphones', 75),
(5, 2, 'WH-B', 'wireless charger', 200);

CREATE INDEX inventory_bm25_idx ON inventory USING bm25 (id, product_name) WITH (key_field = 'id');

-- Composite key join test (region_id AND warehouse_code)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.product_name, w.name AS warehouse_name
FROM inventory i
JOIN warehouses w ON i.region_id = w.region_id AND i.warehouse_code = w.warehouse_code
WHERE i.product_name @@@ 'wireless'
LIMIT 10;

SELECT i.id, i.product_name, w.name AS warehouse_name
FROM inventory i
JOIN warehouses w ON i.region_id = w.region_id AND i.warehouse_code = w.warehouse_code
WHERE i.product_name @@@ 'wireless'
ORDER BY i.id
LIMIT 10;

-- =============================================================================
-- TEST 17: Join key value of 0 (regression test for magic key collision)
-- =============================================================================

-- Create tables where join key value 0 is valid
DROP TABLE IF EXISTS items CASCADE;
DROP TABLE IF EXISTS item_types CASCADE;

CREATE TABLE item_types (
    type_id INTEGER PRIMARY KEY,
    type_name TEXT,
    description TEXT
);

CREATE TABLE items (
    id INTEGER PRIMARY KEY,
    type_id INTEGER,
    name TEXT,
    details TEXT
);

-- Explicitly include type_id = 0
INSERT INTO item_types (type_id, type_name, description) VALUES
(0, 'Uncategorized', 'Items without category'),
(1, 'Electronics', 'Electronic items'),
(2, 'Accessories', 'Accessory items');

INSERT INTO items (id, type_id, name, details) VALUES
(1, 0, 'Mystery Box', 'wireless mystery item'),
(2, 0, 'Unknown Gadget', 'unclassified wireless device'),
(3, 1, 'Smart Speaker', 'wireless bluetooth speaker'),
(4, 2, 'Phone Case', 'protective case');

CREATE INDEX items_bm25_idx ON items USING bm25 (id, name, details) WITH (key_field = 'id');

-- Test that items with type_id = 0 are correctly joined (not treated as cross-join)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, t.type_name
FROM items i
JOIN item_types t ON i.type_id = t.type_id
WHERE i.details @@@ 'wireless'
LIMIT 10;

SELECT i.id, i.name, t.type_name
FROM items i
JOIN item_types t ON i.type_id = t.type_id
WHERE i.details @@@ 'wireless'
ORDER BY i.id
LIMIT 10;

-- Verify type_id = 0 items are joined to 'Uncategorized' type
SELECT i.id, i.name, t.type_name, t.type_id
FROM items i
JOIN item_types t ON i.type_id = t.type_id
WHERE i.type_id = 0
ORDER BY i.id;

-- =============================================================================
-- TEST 18: Memory fallback to nested loop (small work_mem)
-- =============================================================================

-- Save current work_mem and set very small value to trigger fallback
-- Note: This test may still use hash join if the data is small enough
SET work_mem = '64kB';

-- Create larger dataset to potentially trigger memory limit
DROP TABLE IF EXISTS large_orders CASCADE;
DROP TABLE IF EXISTS large_suppliers CASCADE;

CREATE TABLE large_suppliers (
    id SERIAL PRIMARY KEY,
    name TEXT,
    country TEXT
);

CREATE TABLE large_orders (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER,
    description TEXT
);

-- Insert suppliers
INSERT INTO large_suppliers (name, country)
SELECT 
    'Supplier ' || i,
    CASE WHEN i % 3 = 0 THEN 'USA' WHEN i % 3 = 1 THEN 'UK' ELSE 'Germany' END
FROM generate_series(1, 100) i;

-- Insert enough orders to potentially exceed small work_mem
INSERT INTO large_orders (supplier_id, description)
SELECT 
    (i % 100) + 1,
    CASE WHEN i % 5 = 0 THEN 'wireless product order' ELSE 'regular product order' END
FROM generate_series(1, 1000) i;

CREATE INDEX large_orders_bm25_idx ON large_orders USING bm25 (id, description) WITH (key_field = 'id');

-- This query may fall back to nested loop due to small work_mem
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT lo.id, lo.description, ls.name AS supplier_name
FROM large_orders lo
JOIN large_suppliers ls ON lo.supplier_id = ls.id
WHERE lo.description @@@ 'wireless'
LIMIT 10;

SELECT lo.id, lo.description, ls.name AS supplier_name
FROM large_orders lo
JOIN large_suppliers ls ON lo.supplier_id = ls.id
WHERE lo.description @@@ 'wireless'
ORDER BY lo.id
LIMIT 10;

-- Reset work_mem
RESET work_mem;

-- =============================================================================
-- TEST 19: Complex join-level predicate with NOT and OR
-- =============================================================================

-- Complex condition: (p.description @@@ 'wireless' AND NOT p.description @@@ 'mouse') OR s.contact_info @@@ 'shipping'
-- This tests:
-- 1. Negation within a search predicate
-- 2. OR combining predicates across tables
-- 3. AND within a single table's predicate
-- EXPECTED: Products matching 'wireless' but NOT 'mouse' in description,
--           OR any product joined to a supplier with 'shipping' in contact_info
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'wireless' AND NOT p.description @@@ 'mouse') OR s.contact_info @@@ 'shipping'
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'wireless' AND NOT p.description @@@ 'mouse') OR s.contact_info @@@ 'shipping'
ORDER BY p.id
LIMIT 10;

-- Another complex pattern: NOT (p.description @@@ 'cable' OR p.description @@@ 'stand')
-- Products that do NOT contain 'cable' AND do NOT contain 'stand'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE NOT (p.description @@@ 'cable' OR p.description @@@ 'stand')
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE NOT (p.description @@@ 'cable' OR p.description @@@ 'stand')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS customers CASCADE;
DROP TABLE IF EXISTS inventory CASCADE;
DROP TABLE IF EXISTS warehouses CASCADE;
DROP TABLE IF EXISTS items CASCADE;
DROP TABLE IF EXISTS item_types CASCADE;
DROP TABLE IF EXISTS large_orders CASCADE;
DROP TABLE IF EXISTS large_suppliers CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
