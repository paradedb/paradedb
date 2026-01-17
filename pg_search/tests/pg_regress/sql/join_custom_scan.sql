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

-- This is the canonical "Single Feature" join pattern from the TopN spec.
-- Score propagation through JoinScan is supported - paradedb.score() returns
-- the BM25 score from the search predicate on the driving side.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name, paradedb.score(p.id)
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY paradedb.score(p.id) DESC
LIMIT 5;

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
-- TEST 7B: Both side-level AND join-level predicates combined
-- =============================================================================
-- This test shows a query where both sides have side-level predicates AND
-- there's a join-level predicate spanning both tables.
-- Side-level inner: p.description @@@ 'wireless' matches 201,206,207
-- Side-level outer: s.contact_info @@@ 'technology' matches 151
-- Join candidates after side filters: (201,151), (206,151)
-- Join-level: p.name @@@ 'headphones' OR s.name @@@ 'TechCorp'
--   - p.name @@@ 'headphones': matches 206
--   - s.name @@@ 'TechCorp': matches 151
-- Since supplier 151 matches 'TechCorp', both (201,151) and (206,151) pass
-- Expected: 2 rows (201 and 206)

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND s.contact_info @@@ 'technology'
  AND (p.name @@@ 'headphones' OR s.name @@@ 'TechCorp')
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND s.contact_info @@@ 'technology'
  AND (p.name @@@ 'headphones' OR s.name @@@ 'TechCorp')
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
-- TEST 20: Deeply nested boolean expressions
-- =============================================================================

-- Deeply nested: (p_cond1 OR (p_cond2 OR (s_cond AND NOT p_cond3)))
-- This tests the recursive expression tree building
-- p_cond1: p.description @@@ 'keyboard'
-- p_cond2: p.description @@@ 'headphones'  
-- s_cond: s.contact_info @@@ 'shipping'
-- p_cond3: p.description @@@ 'wireless'
-- EXPECTED: Products with 'keyboard' OR 'headphones' OR (supplier has 'shipping' AND product NOT 'wireless')
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'keyboard' OR (p.description @@@ 'headphones' OR (s.contact_info @@@ 'shipping' AND NOT p.description @@@ 'wireless'))
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'keyboard' OR (p.description @@@ 'headphones' OR (s.contact_info @@@ 'shipping' AND NOT p.description @@@ 'wireless'))
ORDER BY p.id
LIMIT 10;

-- AND of multiple single-table predicates combined with OR across tables
-- ((p.description @@@ 'wireless' AND p.description @@@ 'mouse') OR (s.contact_info @@@ 'shipping' AND s.country @@@ 'UK'))
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'wireless' AND p.description @@@ 'mouse') OR (s.contact_info @@@ 'shipping' AND s.country @@@ 'UK')
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'wireless' AND p.description @@@ 'mouse') OR (s.contact_info @@@ 'shipping' AND s.country @@@ 'UK')
ORDER BY p.id
LIMIT 10;

-- Triple-nested NOT: NOT (NOT (NOT p.description @@@ 'cable'))
-- Equivalent to: NOT p.description @@@ 'cable'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE NOT (NOT (NOT p.description @@@ 'cable'))
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE NOT (NOT (NOT p.description @@@ 'cable'))
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 15: Different join key types - TEXT keys
-- =============================================================================
-- Verify JoinScan works with TEXT join keys, not just INTEGER

DROP TABLE IF EXISTS docs CASCADE;
DROP TABLE IF EXISTS authors CASCADE;

CREATE TABLE authors (
    author_code TEXT PRIMARY KEY,
    name TEXT,
    bio TEXT
);

CREATE TABLE docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT,
    author_code TEXT
);

INSERT INTO authors (author_code, name, bio) VALUES
('AUTH001', 'Alice Smith', 'Expert in database systems and search technology'),
('AUTH002', 'Bob Jones', 'Specialist in distributed computing'),
('AUTH003', 'Carol White', 'Focus on machine learning and AI');

INSERT INTO docs (title, content, author_code) VALUES
('Database Indexing', 'Full-text search indexing techniques', 'AUTH001'),
('Search Optimization', 'Optimizing search queries for performance', 'AUTH001'),
('Distributed Systems', 'Building scalable distributed architectures', 'AUTH002'),
('ML Basics', 'Introduction to machine learning concepts', 'AUTH003');

CREATE INDEX docs_bm25_idx ON docs USING bm25 (id, title, content) WITH (key_field = 'id');
CREATE INDEX authors_bm25_idx ON authors USING bm25 (author_code, name, bio) WITH (key_field = 'author_code');

-- JoinScan with TEXT join keys
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.title, a.name
FROM docs d
JOIN authors a ON d.author_code = a.author_code
WHERE d.content @@@ 'search'
LIMIT 10;

SELECT d.title, a.name
FROM docs d
JOIN authors a ON d.author_code = a.author_code
WHERE d.content @@@ 'search'
LIMIT 10;

-- =============================================================================
-- TEST 16: NULL key handling
-- =============================================================================
-- Verify that NULL join keys are correctly excluded (standard SQL semantics)

DROP TABLE IF EXISTS items_with_nulls CASCADE;
DROP TABLE IF EXISTS categories_with_nulls CASCADE;

CREATE TABLE categories_with_nulls (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT
);

CREATE TABLE items_with_nulls (
    id INTEGER PRIMARY KEY,
    name TEXT,
    content TEXT,
    category_id INTEGER  -- Nullable foreign key
);

INSERT INTO categories_with_nulls (id, name, description) VALUES
(1, 'Electronics', 'Electronic devices and gadgets'),
(2, 'Books', 'Physical and digital books'),
(3, 'Clothing', 'Apparel and accessories');

INSERT INTO items_with_nulls (id, name, content, category_id) VALUES
(101, 'Laptop', 'Powerful laptop for programming', 1),
(102, 'Phone', 'Smartphone with great camera', 1),
(103, 'Novel', 'Bestselling fiction novel', 2),
(104, 'Orphan Item', 'Item with no category assignment', NULL),  -- NULL category
(105, 'Another Orphan', 'Another uncategorized item', NULL);     -- NULL category

CREATE INDEX items_nulls_bm25_idx ON items_with_nulls USING bm25 (id, name, content) WITH (key_field = 'id');

-- Query should NOT return items with NULL category_id
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.name AS item_name, c.name AS category_name
FROM items_with_nulls i
JOIN categories_with_nulls c ON i.category_id = c.id
WHERE i.content @@@ 'item OR laptop OR phone OR novel'
LIMIT 10;

-- Should return only rows with non-NULL category_id that match the search
-- The 2 items with NULL category_id are excluded by the JOIN
SELECT i.name AS item_name, c.name AS category_name
FROM items_with_nulls i
JOIN categories_with_nulls c ON i.category_id = c.id
WHERE i.content @@@ 'item OR laptop OR novel'
ORDER BY i.id
LIMIT 10;

-- =============================================================================
-- TEST 17: Cross join (no equi-join keys)
-- =============================================================================
-- Verify JoinScan handles cross joins where there are no equi-join conditions

DROP TABLE IF EXISTS colors CASCADE;
DROP TABLE IF EXISTS sizes CASCADE;

CREATE TABLE colors (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT
);

CREATE TABLE sizes (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT
);

INSERT INTO colors (id, name, description) VALUES
(1, 'Red', 'Bright red color'),
(2, 'Blue', 'Ocean blue color'),
(3, 'Green', 'Forest green color');

INSERT INTO sizes (id, name, description) VALUES
(10, 'Small', 'Small size for compact items'),
(20, 'Medium', 'Medium size for average items'),
(30, 'Large', 'Large size for big items');

CREATE INDEX colors_bm25_idx ON colors USING bm25 (id, name, description) WITH (key_field = 'id');
CREATE INDEX sizes_bm25_idx ON sizes USING bm25 (id, name, description) WITH (key_field = 'id');

-- Cross join with search predicates on both sides
-- Note: This may or may not use JoinScan depending on planner decisions
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.name AS color, s.name AS size
FROM colors c, sizes s
WHERE c.description @@@ 'color' AND s.description @@@ 'size'
LIMIT 10;

SELECT c.name AS color, s.name AS size
FROM colors c, sizes s
WHERE c.description @@@ 'color' AND s.description @@@ 'size'
ORDER BY c.id, s.id
LIMIT 10;

-- =============================================================================
-- TEST 18: Multi-column composite join keys
-- =============================================================================
-- Verify JoinScan handles composite (multi-column) join keys

DROP TABLE IF EXISTS order_items CASCADE;
DROP TABLE IF EXISTS order_details CASCADE;

CREATE TABLE order_details (
    order_id INTEGER,
    line_num INTEGER,
    product_name TEXT,
    description TEXT,
    PRIMARY KEY (order_id, line_num)
);

CREATE TABLE order_items (
    id SERIAL PRIMARY KEY,
    order_id INTEGER,
    line_num INTEGER,
    quantity INTEGER,
    notes TEXT
);

INSERT INTO order_details (order_id, line_num, product_name, description) VALUES
(1, 1, 'Widget A', 'High quality widget for industrial use'),
(1, 2, 'Widget B', 'Standard widget for general purpose'),
(2, 1, 'Gadget X', 'Advanced gadget with wireless connectivity'),
(2, 2, 'Gadget Y', 'Basic gadget for everyday use');

INSERT INTO order_items (order_id, line_num, quantity, notes) VALUES
(1, 1, 10, 'Rush order for wireless widgets'),
(1, 2, 5, 'Standard delivery'),
(2, 1, 3, 'Wireless gadget order'),
(2, 2, 7, 'Bulk order');

CREATE INDEX order_details_bm25_idx ON order_details USING bm25 (order_id, product_name, description) WITH (key_field = 'order_id');
CREATE INDEX order_items_bm25_idx ON order_items USING bm25 (id, notes) WITH (key_field = 'id');

-- Join on composite key (order_id, line_num)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT od.product_name, oi.quantity, oi.notes
FROM order_details od
JOIN order_items oi ON od.order_id = oi.order_id AND od.line_num = oi.line_num
WHERE od.description @@@ 'wireless'
LIMIT 10;

SELECT od.product_name, oi.quantity, oi.notes
FROM order_details od
JOIN order_items oi ON od.order_id = oi.order_id AND od.line_num = oi.line_num
WHERE od.description @@@ 'wireless'
ORDER BY od.order_id, od.line_num
LIMIT 10;

-- =============================================================================
-- TEST 19: Memory overflow - nested loop fallback
-- =============================================================================
-- Verify JoinScan gracefully handles memory overflow by falling back to nested loop
-- Note: This is a functional test, not a stress test. We just verify the query
-- completes correctly even with constrained work_mem.

DROP TABLE IF EXISTS mem_test_products CASCADE;
DROP TABLE IF EXISTS mem_test_suppliers CASCADE;

CREATE TABLE mem_test_suppliers (
    id INTEGER PRIMARY KEY,
    name TEXT,
    info TEXT
);

CREATE TABLE mem_test_products (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    supplier_id INTEGER
);

-- Insert enough data to potentially stress the hash table
-- (actual overflow depends on work_mem setting)
INSERT INTO mem_test_suppliers
SELECT i, 'Supplier ' || i, 'Contact info for supplier ' || i
FROM generate_series(1, 100) AS i;

INSERT INTO mem_test_products
SELECT i, 
       'Product ' || i,
       CASE WHEN i % 3 = 0 THEN 'wireless product' ELSE 'wired product' END,
       (i % 100) + 1
FROM generate_series(1, 500) AS i;

CREATE INDEX mem_test_products_bm25_idx ON mem_test_products 
    USING bm25 (id, name, description) WITH (key_field = 'id');

-- Run with constrained work_mem to test memory handling
-- Note: 64 is the minimum work_mem in PostgreSQL (KB)
SET work_mem = '64kB';

-- This query should still work correctly, whether using hash join or nested loop
SELECT COUNT(*) AS match_count
FROM mem_test_products p
JOIN mem_test_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
LIMIT 100;

-- Verify actual results are correct
SELECT p.name, s.name AS supplier_name
FROM mem_test_products p
JOIN mem_test_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 5;

RESET work_mem;

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
DROP TABLE IF EXISTS docs CASCADE;
DROP TABLE IF EXISTS authors CASCADE;
DROP TABLE IF EXISTS items_with_nulls CASCADE;
DROP TABLE IF EXISTS categories_with_nulls CASCADE;
DROP TABLE IF EXISTS colors CASCADE;
DROP TABLE IF EXISTS sizes CASCADE;
DROP TABLE IF EXISTS order_items CASCADE;
DROP TABLE IF EXISTS order_details CASCADE;
DROP TABLE IF EXISTS mem_test_products CASCADE;
DROP TABLE IF EXISTS mem_test_suppliers CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
