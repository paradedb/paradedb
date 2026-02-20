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
(207, 'Mouse Pad', 'Large gaming mouse pad with wireless charging', 152, 39.69),
(208, 'Cable Organizer', 'Desktop cable organizer for clean setup', 153, 14.99);

-- Create BM25 indexes on both tables
-- Note: JoinScan requires all join key columns and ORDER BY columns to be fast fields
CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, description, supplier_id, price)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}, "price": {"fast": true}}');
CREATE INDEX suppliers_bm25_idx ON suppliers USING bm25 (id, name, contact_info, country)
WITH (key_field = 'id');

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
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
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
-- NOTE: The ORDER-BY column is not in the target list here.
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
ORDER BY paradedb.score(p.id) DESC, p.id
LIMIT 5;

SELECT p.id, p.name, s.name AS supplier_name, paradedb.score(p.id)
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY paradedb.score(p.id) DESC, p.id
LIMIT 5;

-- =============================================================================
-- TEST 6B: SELECT paradedb.score() WITHOUT ORDER BY score
-- =============================================================================

-- This test verifies that paradedb.score() works correctly in SELECT
-- even when ORDER BY is on a different column (not score).
-- This is an edge case where scores must still be computed for output.
-- It works because TopN executor (used when LIMIT is present) always
-- computes scores internally for ranking, regardless of ORDER BY.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name, paradedb.score(p.id) AS score
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 5;

SELECT p.id, p.name, s.name AS supplier_name, paradedb.score(p.id) AS score
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 5;

-- =============================================================================
-- TEST 6C: paradedb.score() from build side (not driving side)
-- =============================================================================

-- This test verifies that paradedb.score() works correctly when it references
-- the BUILD side (the side without the driving predicate), not the driving side.
-- In this query:
-- - p.description @@@ 'wireless' makes products the driving side (streams from Tantivy)
-- - s.contact_info @@@ 'technology' filters the build side (suppliers)
-- - paradedb.score(s.id) requests the score from the build side
-- The build side's score is stored during materialization and returned correctly.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name, paradedb.score(s.id) AS supplier_score
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND s.contact_info @@@ 'technology'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name, paradedb.score(s.id) AS supplier_score
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND s.contact_info @@@ 'technology'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 6D: Both driving side AND build side scores in the same query
-- =============================================================================

-- This test verifies that we can SELECT paradedb.score() from BOTH sides
-- in the same query. This requires:
-- - Computing scores for the driving side during streaming
-- - Computing scores for the build side during materialization
-- - Returning the correct score for each column based on which side it references
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name,
       paradedb.score(p.id) AS product_score,
       paradedb.score(s.id) AS supplier_score
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND s.contact_info @@@ 'technology'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name,
       paradedb.score(p.id) AS product_score,
       paradedb.score(s.id) AS supplier_score
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND s.contact_info @@@ 'technology'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 7: Both sides have search predicates
-- =============================================================================

-- When both sides have @@@ predicates with LIMIT, JoinScan should be proposed
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.contact_info @@@ 'technology'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.contact_info @@@ 'technology'
ORDER BY p.id
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
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND s.contact_info @@@ 'technology'
  AND (p.name @@@ 'headphones' OR s.name @@@ 'TechCorp')
ORDER BY p.id
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
ORDER BY p.id
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
-- Note: The join between products and categories is on category_id.
-- category_id was added via ALTER TABLE but was NOT added to the BM25 index on products.
-- Therefore, the JoinScan cannot push down the join between products and categories
-- because the join key is not a fast field. The JoinScan should fall back to a
-- standard join for that level.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name, c.name AS category_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
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
ORDER BY p.id
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
ORDER BY p.id
LIMIT 3;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'mouse'
ORDER BY p.id
LIMIT 3;

-- LIMIT with ORDER BY on fast field column (price is a fast field)
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

-- Note: orders.customer_code must be a fast field for the join key
CREATE INDEX orders_bm25_idx ON orders USING bm25 (id, description, customer_code)
WITH (key_field = 'id', text_fields = '{"customer_code": {"fast": true, "tokenizer": {"type": "keyword"}}}');
CREATE INDEX customers_bm25_idx ON customers USING bm25 (customer_code, name, email) WITH (key_field = 'customer_code');

-- TEXT join key test
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT o.id, o.description, c.name AS customer_name
FROM orders o
JOIN customers c ON o.customer_code = c.customer_code
WHERE o.description @@@ 'wireless'
ORDER BY o.id
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

-- Note: inventory needs region_id and warehouse_code as fast fields for composite join keys
CREATE INDEX inventory_bm25_idx ON inventory USING bm25 (id, product_name, region_id, warehouse_code)
WITH (key_field = 'id', numeric_fields = '{"region_id": {"fast": true}}',
      text_fields = '{"warehouse_code": {"fast": true, "tokenizer": {"type": "keyword"}}}');
CREATE INDEX warehouses_bm25_idx ON warehouses USING bm25 (region_id, warehouse_code, name, description)
WITH (key_field = 'region_id',
      text_fields = '{"warehouse_code": {"fast": true, "tokenizer": {"type": "keyword"}}}');

-- Composite key join test (region_id AND warehouse_code)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.product_name, w.name AS warehouse_name
FROM inventory i
JOIN warehouses w ON i.region_id = w.region_id AND i.warehouse_code = w.warehouse_code
WHERE i.product_name @@@ 'wireless'
ORDER BY i.id
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

-- Note: items.type_id must be a fast field for the join key
CREATE INDEX items_bm25_idx ON items USING bm25 (id, name, details, type_id)
WITH (key_field = 'id', numeric_fields = '{"type_id": {"fast": true}}');
CREATE INDEX item_types_bm25_idx ON item_types USING bm25 (type_id, type_name, description) WITH (key_field = 'type_id');

-- Test that items with type_id = 0 are correctly joined (not treated as cross-join)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, t.type_name
FROM items i
JOIN item_types t ON i.type_id = t.type_id
WHERE i.details @@@ 'wireless'
ORDER BY i.id
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
-- TEST 18: Memory Limit Enforcement (Expect OOM)
-- =============================================================================

-- Save current work_mem and set very small value to trigger OOM
-- Note: This verifies that we enforce memory limits and error out because spilling is not implemented
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

-- Note: large_orders.supplier_id must be a fast field for the join key
CREATE INDEX large_orders_bm25_idx ON large_orders USING bm25 (id, description, supplier_id)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}}');
CREATE INDEX large_suppliers_bm25_idx ON large_suppliers USING bm25 (id, name, country) WITH (key_field = 'id');

-- This query may fall back to nested loop due to small work_mem
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT lo.id, lo.description, ls.name AS supplier_name
FROM large_orders lo
JOIN large_suppliers ls ON lo.supplier_id = ls.id
WHERE lo.description @@@ 'wireless'
ORDER BY lo.id
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
ORDER BY p.id
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
ORDER BY p.id
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
ORDER BY p.id
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
ORDER BY p.id
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
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE NOT (NOT (NOT p.description @@@ 'cable'))
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 21: Different join key types - TEXT keys
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

-- Note: docs.author_code must be a fast field for the join key
CREATE INDEX docs_bm25_idx ON docs USING bm25 (id, title, content, author_code)
WITH (key_field = 'id', text_fields = '{"author_code": {"fast": true, "tokenizer": {"type": "keyword"}}}');
CREATE INDEX authors_bm25_idx ON authors USING bm25 (author_code, name, bio) WITH (key_field = 'author_code');

-- JoinScan with TEXT join keys
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.title, a.name
FROM docs d
JOIN authors a ON d.author_code = a.author_code
WHERE d.content @@@ 'search'
ORDER BY d.id
LIMIT 10;

SELECT d.title, a.name
FROM docs d
JOIN authors a ON d.author_code = a.author_code
WHERE d.content @@@ 'search'
ORDER BY d.id
LIMIT 10;

-- =============================================================================
-- TEST 22: NULL key handling
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

-- Note: items.category_id must be a fast field for the join key
CREATE INDEX items_nulls_bm25_idx ON items_with_nulls USING bm25 (id, name, content, category_id)
WITH (key_field = 'id', numeric_fields = '{"category_id": {"fast": true}}');
CREATE INDEX categories_nulls_bm25_idx ON categories_with_nulls USING bm25 (id, name, description) WITH (key_field = 'id');

-- Query should NOT return items with NULL category_id
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.name AS item_name, c.name AS category_name
FROM items_with_nulls i
JOIN categories_with_nulls c ON i.category_id = c.id
WHERE i.content @@@ 'item OR laptop OR novel'
ORDER BY i.id
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
-- TEST 23: Cross join (no equi-join keys) - JoinScan NOT proposed
-- =============================================================================
-- Verify JoinScan does NOT handle cross joins (no equi-join conditions).
-- Cross joins require O(N*M) comparisons and are better handled by PostgreSQL.

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
-- JoinScan should NOT be proposed - falls back to PostgreSQL's Nested Loop
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.name AS color, s.name AS size
FROM colors c, sizes s
WHERE c.description @@@ 'color' AND s.description @@@ 'size'
ORDER BY c.id, s.id
LIMIT 10;

-- =============================================================================
-- TEST 24: Multi-column composite join keys
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

-- Note: Both tables need order_id and line_num as fast fields for composite join keys
CREATE INDEX order_details_bm25_idx ON order_details USING bm25 (order_id, product_name, description, line_num)
WITH (key_field = 'order_id', numeric_fields = '{"line_num": {"fast": true}}');
CREATE INDEX order_items_bm25_idx ON order_items USING bm25 (id, notes, order_id, line_num)
WITH (key_field = 'id', numeric_fields = '{"order_id": {"fast": true}, "line_num": {"fast": true}}');

-- Join on composite key (order_id, line_num)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT od.product_name, oi.quantity, oi.notes
FROM order_details od
JOIN order_items oi ON od.order_id = oi.order_id AND od.line_num = oi.line_num
WHERE od.description @@@ 'wireless'
ORDER BY od.order_id, od.line_num
LIMIT 10;

SELECT od.product_name, oi.quantity, oi.notes
FROM order_details od
JOIN order_items oi ON od.order_id = oi.order_id AND od.line_num = oi.line_num
WHERE od.description @@@ 'wireless'
ORDER BY od.order_id, od.line_num
LIMIT 10;

-- =============================================================================
-- TEST 25: Memory Limit Enforcement (Expect OOM)
-- =============================================================================
-- Verify JoinScan handles memory overflow by erroring out (OOM)
-- Note: This is a functional test to ensure we don't crash when memory is exceeded.
-- Since spilling is not implemented, we expect an OOM error.

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

-- Note: mem_test_products.supplier_id must be a fast field for the join key
CREATE INDEX mem_test_products_bm25_idx ON mem_test_products 
    USING bm25 (id, name, description, supplier_id)
    WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}}');
CREATE INDEX mem_test_suppliers_bm25_idx ON mem_test_suppliers
    USING bm25 (id, name, info) WITH (key_field = 'id');

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
-- TEST 26: UUID join keys
-- =============================================================================
-- Verify JoinScan works with UUID join keys

DROP TABLE IF EXISTS uuid_orders CASCADE;
DROP TABLE IF EXISTS uuid_customers CASCADE;

CREATE TABLE uuid_customers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT,
    email TEXT
);

CREATE TABLE uuid_orders (
    id SERIAL PRIMARY KEY,
    customer_id UUID,
    description TEXT,
    amount NUMERIC(10,2)
);

-- Insert with explicit UUIDs for reproducibility
INSERT INTO uuid_customers (id, name, email) VALUES
('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'Alice', 'alice@example.com'),
('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a22', 'Bob', 'bob@example.com'),
('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380a33', 'Carol', 'carol@example.com');

INSERT INTO uuid_orders (customer_id, description, amount) VALUES
('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'Wireless keyboard order', 99.99),
('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'USB hub purchase', 29.99),
('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a22', 'Monitor stand order', 49.99),
('c0eebc99-9c0b-4ef8-bb6d-6bb9bd380a33', 'Wireless mouse order', 39.69);

-- Note: uuid_orders.customer_id must be a fast field for the join key
-- UUID columns use key_field which is implicitly fast, or explicit text_fields config
CREATE INDEX uuid_orders_bm25_idx ON uuid_orders USING bm25 (id, description, customer_id)
WITH (key_field = 'id', text_fields = '{"customer_id": {"fast": true, "tokenizer": {"type": "keyword"}}}');
-- uuid_customers.id is the key_field, which is implicitly fast
CREATE INDEX uuid_customers_bm25_idx ON uuid_customers USING bm25 (id, name, email) WITH (key_field = 'id');

-- JoinScan with UUID join keys
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT o.description, c.name
FROM uuid_orders o
JOIN uuid_customers c ON o.customer_id = c.id
WHERE o.description @@@ 'wireless'
ORDER BY o.id
LIMIT 10;

SELECT o.description, c.name
FROM uuid_orders o
JOIN uuid_customers c ON o.customer_id = c.id
WHERE o.description @@@ 'wireless'
ORDER BY o.id
LIMIT 10;

-- =============================================================================
-- TEST 27: NUMERIC join keys
-- =============================================================================
-- Verify JoinScan works with NUMERIC (decimal) join keys

DROP TABLE IF EXISTS numeric_transactions CASCADE;
DROP TABLE IF EXISTS numeric_accounts CASCADE;

CREATE TABLE numeric_accounts (
    account_num NUMERIC(20,0) PRIMARY KEY,
    holder_name TEXT,
    account_type TEXT
);

CREATE TABLE numeric_transactions (
    id SERIAL PRIMARY KEY,
    account_num NUMERIC(20,0),
    description TEXT,
    amount NUMERIC(15,2)
);

INSERT INTO numeric_accounts (account_num, holder_name, account_type) VALUES
(12345678901234567890, 'John Doe', 'Checking'),
(98765432109876543210, 'Jane Smith', 'Savings'),
(11111111111111111111, 'Bob Wilson', 'Investment');

INSERT INTO numeric_transactions (account_num, description, amount) VALUES
(12345678901234567890, 'Wire transfer received', 1000.00),
(12345678901234567890, 'ATM withdrawal', -200.00),
(98765432109876543210, 'Interest payment', 50.00),
(11111111111111111111, 'Stock purchase wire', 5000.00);

-- Note: numeric_transactions.account_num must be a fast field for the join key
CREATE INDEX numeric_trans_bm25_idx ON numeric_transactions USING bm25 (id, description, account_num)
WITH (key_field = 'id', numeric_fields = '{"account_num": {"fast": true}}');
-- numeric_accounts.account_num is the key_field, which is implicitly fast
CREATE INDEX numeric_accounts_bm25_idx ON numeric_accounts USING bm25 (account_num, holder_name, account_type)
WITH (key_field = 'account_num');

-- JoinScan with NUMERIC join keys
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT t.description, a.holder_name, t.amount
FROM numeric_transactions t
JOIN numeric_accounts a ON t.account_num = a.account_num
WHERE t.description @@@ 'wire'
ORDER BY t.id
LIMIT 10;

-- =============================================================================
-- TEST 28: Large result set (functional, not performance)
-- =============================================================================
-- Verify JoinScan handles larger result sets correctly
-- This is a functional test, not a benchmark

DROP TABLE IF EXISTS large_items CASCADE;
DROP TABLE IF EXISTS large_categories CASCADE;

CREATE TABLE large_categories (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT
);

CREATE TABLE large_items (
    id SERIAL PRIMARY KEY,
    name TEXT,
    content TEXT,
    category_id INTEGER
);

-- Insert 50 categories
INSERT INTO large_categories
SELECT i, 'Category ' || i, 'Description for category ' || i
FROM generate_series(1, 50) AS i;

-- Insert 1000 items distributed across categories
INSERT INTO large_items (name, content, category_id)
SELECT 
    'Item ' || i,
    CASE 
        WHEN i % 5 = 0 THEN 'wireless product with bluetooth'
        WHEN i % 7 = 0 THEN 'cable product with usb connector'
        ELSE 'standard product item'
    END,
    (i % 50) + 1
FROM generate_series(1, 1000) AS i;

-- Note: large_items.category_id must be a fast field for the join key
CREATE INDEX large_items_bm25_idx ON large_items USING bm25 (id, name, content, category_id)
WITH (key_field = 'id', numeric_fields = '{"category_id": {"fast": true}}');
CREATE INDEX large_categories_bm25_idx ON large_categories USING bm25 (id, name, description) WITH (key_field = 'id');

-- Query with larger LIMIT to test larger result sets
SELECT COUNT(*) AS wireless_count
FROM large_items li
JOIN large_categories lc ON li.category_id = lc.id
WHERE li.content @@@ 'wireless'
LIMIT 500;

-- Verify first few results
SELECT li.name, lc.name AS category_name
FROM large_items li
JOIN large_categories lc ON li.category_id = lc.id
WHERE li.content @@@ 'wireless'
ORDER BY li.id
LIMIT 5;

-- =============================================================================
-- TEST 29: Visibility after multiple UPDATEs
-- =============================================================================
-- Verify JoinScan handles visibility correctly after multiple UPDATE cycles
-- Note: True concurrent update testing requires multiple connections,
-- which is not possible in a single regression test. This tests sequential
-- UPDATE visibility instead.

DROP TABLE IF EXISTS update_test_items CASCADE;
DROP TABLE IF EXISTS update_test_refs CASCADE;

CREATE TABLE update_test_refs (
    id INTEGER PRIMARY KEY,
    ref_name TEXT
);

CREATE TABLE update_test_items (
    id INTEGER PRIMARY KEY,
    content TEXT,
    ref_id INTEGER,
    version INTEGER DEFAULT 1
);

INSERT INTO update_test_refs VALUES (1, 'Ref A'), (2, 'Ref B'), (3, 'Ref C');

INSERT INTO update_test_items (id, content, ref_id) VALUES
(101, 'wireless device alpha', 1),
(102, 'wired device beta', 2),
(103, 'wireless device gamma', 3);

-- Note: update_test_items.ref_id must be a fast field for the join key
CREATE INDEX update_items_bm25_idx ON update_test_items USING bm25 (id, content, ref_id)
WITH (key_field = 'id', numeric_fields = '{"ref_id": {"fast": true}}');
CREATE INDEX update_refs_bm25_idx ON update_test_refs USING bm25 (id, ref_name) WITH (key_field = 'id');

-- Initial query
SELECT i.id, i.content, r.ref_name, i.version
FROM update_test_items i
JOIN update_test_refs r ON i.ref_id = r.id
WHERE i.content @@@ 'wireless'
ORDER BY i.id
LIMIT 10;

-- First UPDATE cycle
UPDATE update_test_items SET version = 2 WHERE content LIKE '%wireless%';

-- Query after first update - should still find wireless items
SELECT i.id, i.content, r.ref_name, i.version
FROM update_test_items i
JOIN update_test_refs r ON i.ref_id = r.id
WHERE i.content @@@ 'wireless'
ORDER BY i.id
LIMIT 10;

-- Second UPDATE cycle - change content
UPDATE update_test_items SET content = 'updated wireless device', version = 3 WHERE id = 101;

-- Query after content update
SELECT i.id, i.content, r.ref_name, i.version
FROM update_test_items i
JOIN update_test_refs r ON i.ref_id = r.id
WHERE i.content @@@ 'wireless'
ORDER BY i.id
LIMIT 10;

-- Third UPDATE cycle - change ref_id (join key)
UPDATE update_test_items SET ref_id = 2, version = 4 WHERE id = 103;

-- Query after join key update
SELECT i.id, i.content, r.ref_name, i.version
FROM update_test_items i
JOIN update_test_refs r ON i.ref_id = r.id
WHERE i.content @@@ 'wireless'
ORDER BY i.id
LIMIT 10;

-- =============================================================================
-- TEST 30: Qgen-style setup - Index before data with NOT operator
-- =============================================================================
-- This test replicates the qgen test setup which revealed a bug:
-- 1. Create index BEFORE inserting data (creates multiple segments)
-- 2. Use NOT operator in WHERE clause
-- 3. Join on non-PK column
-- 4. Larger dataset (100 rows)
-- Error was: "could not read blocks 65536..65536: read only 0 of 8192 bytes"

DROP TABLE IF EXISTS qgen_products CASCADE;
DROP TABLE IF EXISTS qgen_users CASCADE;

-- Use SERIAL8 (bigint) like the qgen test to verify the fix
CREATE TABLE qgen_users (
    id SERIAL8 PRIMARY KEY,
    uuid UUID,
    name TEXT,
    color TEXT,
    age INTEGER,
    quantity INTEGER,
    price NUMERIC(10,2),
    rating INTEGER
);

CREATE TABLE qgen_products (
    id SERIAL8 PRIMARY KEY,
    uuid UUID,
    name TEXT,
    color TEXT,
    age INTEGER,
    quantity INTEGER,
    price NUMERIC(10,2),
    rating INTEGER
);

-- Create index BEFORE inserting data (this is the key difference from other tests)
-- This causes multiple segments to be created as data is inserted
-- Note: age must be a fast field for the join key
CREATE INDEX qgen_users_bm25_idx ON qgen_users USING bm25 (id, name, age) WITH (
    key_field = 'id',
    text_fields = '{ "name": { "tokenizer": { "type": "keyword" }, "fast": true } }',
    numeric_fields = '{ "age": { "fast": true } }'
);

CREATE INDEX qgen_products_bm25_idx ON qgen_products USING bm25 (id, name, age) WITH (
    key_field = 'id',
    text_fields = '{ "name": { "tokenizer": { "type": "keyword" }, "fast": true } }',
    numeric_fields = '{ "age": { "fast": true } }'
);

-- Insert sample value first
INSERT INTO qgen_users (uuid, name, color, age, quantity, price, rating) 
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'bob', 'blue', 20, 7, 99.99, 4);

INSERT INTO qgen_products (uuid, name, color, age, quantity, price, rating) 
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'bob', 'blue', 20, 7, 99.99, 4);

-- Insert random data (100 rows each)
-- Using deterministic seed for reproducibility
SELECT setseed(0.5);

INSERT INTO qgen_users (uuid, name, color, age, quantity, price, rating)
SELECT 
    rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid,
    (ARRAY ['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy']::text[])[(floor(random() * 7) + 1)::int],
    (ARRAY ['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow', NULL]::text[])[(floor(random() * 8) + 1)::int],
    (floor(random() * 100) + 1)::int,
    CASE WHEN random() < 0.1 THEN NULL ELSE (floor(random() * 100) + 1)::int END,
    (random() * 1000 + 10)::numeric(10,2),
    (floor(random() * 5) + 1)::int
FROM generate_series(1, 100);

INSERT INTO qgen_products (uuid, name, color, age, quantity, price, rating)
SELECT 
    rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid,
    (ARRAY ['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy']::text[])[(floor(random() * 7) + 1)::int],
    (ARRAY ['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow', NULL]::text[])[(floor(random() * 8) + 1)::int],
    (floor(random() * 100) + 1)::int,
    CASE WHEN random() < 0.1 THEN NULL ELSE (floor(random() * 100) + 1)::int END,
    (random() * 1000 + 10)::numeric(10,2),
    (floor(random() * 5) + 1)::int
FROM generate_series(1, 100);

ANALYZE qgen_users;
ANALYZE qgen_products;

-- TEST 30A: Simple query without NOT (baseline - should work)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE qgen_users.name @@@ 'bob' 
ORDER BY qgen_users.id 
LIMIT 5;

SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE qgen_users.name @@@ 'bob' 
ORDER BY qgen_users.id 
LIMIT 5;

-- TEST 30B: Query with NOT operator (this is where the bug occurred)
-- Error was: "could not read blocks 65536..65536: read only 0 of 8192 bytes"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE NOT (qgen_users.name @@@ 'bob') 
ORDER BY qgen_users.id 
LIMIT 5;

SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE NOT (qgen_users.name @@@ 'bob') 
ORDER BY qgen_users.id 
LIMIT 5;

-- TEST 30C: OR with predicates spanning both tables
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE (qgen_products.name @@@ 'alice') OR (qgen_users.name @@@ 'bob')
ORDER BY qgen_users.id 
LIMIT 5;

SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE (qgen_products.name @@@ 'alice') OR (qgen_users.name @@@ 'bob')
ORDER BY qgen_users.id 
LIMIT 5;

-- =============================================================================
-- TEST 31: Execution hints - small build side (nested loop preference)
-- =============================================================================
-- This test verifies that execution hints work for very small joins.
-- When estimated_build_rows < 10, the planner hints to prefer nested loop
-- to avoid hash table overhead.

DROP TABLE IF EXISTS tiny_products CASCADE;
DROP TABLE IF EXISTS tiny_refs CASCADE;

CREATE TABLE tiny_refs (
    id INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE tiny_products (
    id INTEGER PRIMARY KEY,
    ref_id INTEGER,
    description TEXT
);

-- Very small build side (only 3 rows)
INSERT INTO tiny_refs VALUES (1, 'Ref A'), (2, 'Ref B'), (3, 'Ref C');

INSERT INTO tiny_products VALUES
(101, 1, 'wireless device alpha'),
(102, 2, 'wired device beta'),
(103, 1, 'wireless device gamma');

-- Note: tiny_products.ref_id must be a fast field for the join key
CREATE INDEX tiny_products_bm25_idx ON tiny_products USING bm25 (id, description, ref_id)
WITH (key_field = 'id', numeric_fields = '{"ref_id": {"fast": true}}');
CREATE INDEX tiny_refs_bm25_idx ON tiny_refs USING bm25 (id, name) WITH (key_field = 'id');

-- Query with very small build side - should work correctly regardless of algorithm
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT tp.id, tp.description, tr.name
FROM tiny_products tp
JOIN tiny_refs tr ON tp.ref_id = tr.id
WHERE tp.description @@@ 'wireless'
ORDER BY tp.id
LIMIT 10;

SELECT tp.id, tp.description, tr.name
FROM tiny_products tp
JOIN tiny_refs tr ON tp.ref_id = tr.id
WHERE tp.description @@@ 'wireless'
ORDER BY tp.id
LIMIT 10;

-- =============================================================================
-- TEST 32: Execution hints - verify hash table pre-sizing (functional test)
-- =============================================================================
-- This test verifies that the execution hints system works with larger datasets.
-- The planner should estimate build rows and pass hints to the executor.

DROP TABLE IF EXISTS hint_test_products CASCADE;
DROP TABLE IF EXISTS hint_test_categories CASCADE;

CREATE TABLE hint_test_categories (
    id INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE hint_test_products (
    id INTEGER PRIMARY KEY,
    category_id INTEGER,
    description TEXT
);

-- Medium-sized build side (50 rows)
INSERT INTO hint_test_categories
SELECT i, 'Category ' || i
FROM generate_series(1, 50) i;

-- Products referencing categories
INSERT INTO hint_test_products
SELECT i, (i % 50) + 1, 
    CASE WHEN i % 3 = 0 THEN 'wireless product' ELSE 'standard product' END
FROM generate_series(1, 200) i;

-- Note: hint_test_products.category_id must be a fast field for the join key
CREATE INDEX hint_test_products_bm25_idx ON hint_test_products USING bm25 (id, description, category_id)
WITH (key_field = 'id', numeric_fields = '{"category_id": {"fast": true}}');
CREATE INDEX hint_test_categories_bm25_idx ON hint_test_categories USING bm25 (id, name) WITH (key_field = 'id');

-- Query that exercises hash table with medium build side
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT hp.id, hp.description, hc.name AS category_name
FROM hint_test_products hp
JOIN hint_test_categories hc ON hp.category_id = hc.id
WHERE hp.description @@@ 'wireless'
ORDER BY hp.id
LIMIT 20;

SELECT hp.id, hp.description, hc.name AS category_name
FROM hint_test_products hp
JOIN hint_test_categories hc ON hp.category_id = hc.id
WHERE hp.description @@@ 'wireless'
ORDER BY hp.id
LIMIT 20;

-- Verify count is correct
SELECT COUNT(*) AS wireless_count
FROM hint_test_products hp
JOIN hint_test_categories hc ON hp.category_id = hc.id
WHERE hp.description @@@ 'wireless';

-- =============================================================================
-- TEST 33: Multi-table predicates with fast fields
-- =============================================================================
-- This test demonstrates JoinScan handling multi-table predicates (conditions
-- that reference columns from both tables) when ALL referenced columns are
-- fast fields in their respective BM25 indexes.
--
-- Multi-table predicates like `p.price >= s.min_order_value` are supported
-- when both `price` and `min_order_value` are indexed as fast fields.
-- If any column is NOT a fast field, JoinScan will not be proposed.

-- Add min_order_value to suppliers for cross-relation comparison
ALTER TABLE suppliers ADD COLUMN min_order_value DECIMAL(10,2) DEFAULT 0;
UPDATE suppliers SET min_order_value = 50.00 WHERE id = 151;  -- TechCorp
UPDATE suppliers SET min_order_value = 15.00 WHERE id = 152;  -- GlobalSupply
UPDATE suppliers SET min_order_value = 30.00 WHERE id = 153;  -- FastParts
UPDATE suppliers SET min_order_value = 100.00 WHERE id = 154; -- QualityFirst

-- Recreate indexes with price, min_order_value, and join key columns as fast fields
DROP INDEX IF EXISTS products_bm25_idx;
DROP INDEX IF EXISTS suppliers_bm25_idx;

CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, description, supplier_id, price)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}, "price": {"fast": true}}');

CREATE INDEX suppliers_bm25_idx ON suppliers USING bm25 (id, name, contact_info, country, min_order_value)
WITH (key_field = 'id', numeric_fields = '{"min_order_value": {"fast": true}}');

-- Test case: Search predicate AND multi-table predicate (both columns are fast fields)
-- Products where description matches 'wireless' AND price >= supplier's min_order_value
-- JoinScan SHOULD be proposed because price and min_order_value are fast fields
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, p.price, s.name as supplier, s.min_order_value
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND p.price >= s.min_order_value
ORDER BY p.id
LIMIT 10;

-- Test case: Search predicate OR multi-table predicate (unified expression tree)
-- Products where EITHER description matches 'cable' OR price >= supplier's min_order_value
-- JoinScan SHOULD be proposed because all columns in the multi-table predicate are fast fields
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, p.price, s.name as supplier, s.min_order_value
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'cable'
   OR p.price >= s.min_order_value
LIMIT 10;

-- Verify correct results
SELECT p.id, p.name, p.price, s.name as supplier, s.min_order_value
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'cable'
   OR p.price >= s.min_order_value
ORDER BY p.id
LIMIT 10;

-- Test case: Multi-table predicate with NON-indexed column
-- This should NOT use JoinScan because category_id is not in the BM25 index
-- (demonstrating the rejection of non-fast-field multi-table predicates)
-- Note: category_id was added via ALTER TABLE but is NOT in the recreated index
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name as supplier
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND p.category_id > s.id  -- category_id is NOT in the BM25 index
LIMIT 10;

-- =============================================================================
-- TEST 34: Mixed-case column names (regression test for quoting issues)
-- =============================================================================
-- Verify JoinScan handles mixed-case column names correctly in join keys and sort
DROP TABLE IF EXISTS "MixedCaseTable" CASCADE;

CREATE TABLE "MixedCaseTable" (
    "ID" INTEGER PRIMARY KEY,
    "Content" TEXT,
    "JoinKey" INTEGER
);

-- Note: Suppliers table exists from setup (IDs 151-154)
INSERT INTO "MixedCaseTable" ("ID", "Content", "JoinKey") VALUES (1, 'wireless', 151);

-- Note: "JoinKey" must be a fast field
CREATE INDEX mixed_case_bm25_idx ON "MixedCaseTable" USING bm25 ("ID", "Content", "JoinKey")
WITH (key_field = 'ID', numeric_fields = '{"JoinKey": {"fast": true}}');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT m."Content", s.name
FROM "MixedCaseTable" m
JOIN suppliers s ON m."JoinKey" = s.id
WHERE m."Content" @@@ 'wireless'
ORDER BY m."ID"
LIMIT 5;

SELECT m."Content", s.name
FROM "MixedCaseTable" m
JOIN suppliers s ON m."JoinKey" = s.id
WHERE m."Content" @@@ 'wireless'
ORDER BY m."ID"
LIMIT 5;

DROP TABLE "MixedCaseTable";

-- =============================================================================
-- TEST 35A: Multi-table join - Star Schema (3 tables)
-- =============================================================================

-- Setup specific data for these tests
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;
DROP TABLE IF EXISTS categories CASCADE;

-- Create test tables
CREATE TABLE categories (
    id INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY,
    name TEXT,
    contact_info TEXT,
    country TEXT
);

CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    supplier_id INTEGER,
    category_id INTEGER,
    price DECIMAL(10,2)
);

-- Insert test data
INSERT INTO categories (id, name) VALUES
(10, 'Electronics'),
(11, 'Accessories'),
(12, 'Office');

INSERT INTO suppliers (id, name, contact_info, country) VALUES
(151, 'TechCorp', 'contact@techcorp.com wireless technology', 'USA'),
(152, 'GlobalSupply', 'info@globalsupply.com international shipping', 'UK'),
(153, 'FastParts', 'sales@fastparts.com quick delivery', 'Germany');

INSERT INTO products (id, name, description, supplier_id, category_id, price) VALUES
(201, 'Wireless Mouse', 'Ergonomic wireless mouse', 151, 11, 29.99),
(202, 'USB Cable', 'High-speed USB-C cable', 152, 11, 9.99),
(203, 'Keyboard', 'Mechanical keyboard', 151, 10, 89.99),
(204, 'Monitor Stand', 'Adjustable monitor stand', 153, 12, 49.99),
(206, 'Headphones', 'Wireless noise-canceling headphones', 151, 10, 199.99),
(207, 'Mouse Pad', 'Large gaming mouse pad', 152, 11, 29.99);

-- Create BM25 indexes
CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, description, supplier_id, category_id, price)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}, "category_id": {"fast": true}, "price": {"fast": true}}');

CREATE INDEX suppliers_bm25_idx ON suppliers USING bm25 (id, name, contact_info, country)
WITH (key_field = 'id');

CREATE INDEX categories_bm25_idx ON categories USING bm25 (id, name)
WITH (key_field = 'id');

-- Enable JoinScan
SET paradedb.enable_join_custom_scan = on;

-- Query joining Products, Suppliers, and Categories.
-- Search predicate on Products.
-- Should produce nested JoinScans.

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.name AS product, s.name AS supplier, c.name AS category
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 10;

SELECT p.name AS product, s.name AS supplier, c.name AS category
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 10;

-- Search predicate on Suppliers.
-- Products joins Suppliers, then Categories.

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.name AS product, s.name AS supplier, c.name AS category
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE s.contact_info @@@ 'wireless'
ORDER BY p.id
LIMIT 10;

SELECT p.name AS product, s.name AS supplier, c.name AS category
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE s.contact_info @@@ 'wireless'
ORDER BY p.id
LIMIT 10;

-- Order by score from the nested relation (Products).
-- Products is in the child join (p join c).
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.name, paradedb.score(p.id)
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE p.description @@@ 'wireless'
ORDER BY paradedb.score(p.id) DESC
LIMIT 5;

SELECT p.name, paradedb.score(p.id)
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE p.description @@@ 'wireless'
ORDER BY paradedb.score(p.id) DESC
LIMIT 5;

-- Order by score from the top outer relation (Suppliers).
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT s.name, paradedb.score(s.id)
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE s.contact_info @@@ 'wireless'
ORDER BY paradedb.score(s.id) DESC
LIMIT 5;

SELECT s.name, paradedb.score(s.id)
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
JOIN categories c ON p.category_id = c.id
WHERE s.contact_info @@@ 'wireless'
ORDER BY paradedb.score(s.id) DESC
LIMIT 5;

-- =============================================================================
-- TEST 35B: Multi-table join - Chain Schema (4 tables)
-- =============================================================================

DROP TABLE IF EXISTS level1 CASCADE;
DROP TABLE IF EXISTS level2 CASCADE;
DROP TABLE IF EXISTS level3 CASCADE;
DROP TABLE IF EXISTS level4 CASCADE;

CREATE TABLE level1 (id INTEGER PRIMARY KEY, l2_id INTEGER, name TEXT);
CREATE TABLE level2 (id INTEGER PRIMARY KEY, l3_id INTEGER, name TEXT);
CREATE TABLE level3 (id INTEGER PRIMARY KEY, l4_id INTEGER, name TEXT);
CREATE TABLE level4 (id INTEGER PRIMARY KEY, name TEXT, description TEXT);

INSERT INTO level4 VALUES (1, 'L4-A', 'Deepest level item');
INSERT INTO level3 VALUES (1, 1, 'L3-A');
INSERT INTO level2 VALUES (1, 1, 'L2-A');
INSERT INTO level1 VALUES (1, 1, 'L1-A');

INSERT INTO level4 VALUES (2, 'L4-B', 'Another deep item');
INSERT INTO level3 VALUES (2, 2, 'L3-B');
INSERT INTO level2 VALUES (2, 2, 'L2-B');
INSERT INTO level1 VALUES (2, 2, 'L1-B');

CREATE INDEX l1_bm25 ON level1 USING bm25 (id, l2_id, name) WITH (key_field='id', numeric_fields='{"l2_id": {"fast": true}}');
CREATE INDEX l2_bm25 ON level2 USING bm25 (id, l3_id, name) WITH (key_field='id', numeric_fields='{"l3_id": {"fast": true}}');
CREATE INDEX l3_bm25 ON level3 USING bm25 (id, l4_id, name) WITH (key_field='id', numeric_fields='{"l4_id": {"fast": true}}');
CREATE INDEX l4_bm25 ON level4 USING bm25 (id, name, description) WITH (key_field='id');

-- Join 4 tables, driving predicate on level4
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT l1.name, l2.name, l3.name, l4.name
FROM level1 l1
JOIN level2 l2 ON l1.l2_id = l2.id
JOIN level3 l3 ON l2.l3_id = l3.id
JOIN level4 l4 ON l3.l4_id = l4.id
WHERE l4.description @@@ 'deepest'
ORDER BY l1.id
LIMIT 5;

SELECT l1.name, l2.name, l3.name, l4.name
FROM level1 l1
JOIN level2 l2 ON l1.l2_id = l2.id
JOIN level3 l3 ON l2.l3_id = l3.id
JOIN level4 l4 ON l3.l4_id = l4.id
WHERE l4.description @@@ 'deepest'
ORDER BY l1.id
LIMIT 5;

-- =============================================================================
-- TEST 35C: Chain Schema - Mixed Predicates
-- =============================================================================

-- Predicates on level1 (outermost) and level4 (innermost)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT l1.name, l4.name
FROM level1 l1
JOIN level2 l2 ON l1.l2_id = l2.id
JOIN level3 l3 ON l2.l3_id = l3.id
JOIN level4 l4 ON l3.l4_id = l4.id
WHERE l1.name @@@ 'L1-A' AND l4.description @@@ 'deepest'
ORDER BY l1.id
LIMIT 5;

SELECT l1.name, l4.name
FROM level1 l1
JOIN level2 l2 ON l1.l2_id = l2.id
JOIN level3 l3 ON l2.l3_id = l3.id
JOIN level4 l4 ON l3.l4_id = l4.id
WHERE l1.name @@@ 'L1-A' AND l4.description @@@ 'deepest'
ORDER BY l1.id
LIMIT 5;

-- Predicates on intermediate levels (level2 and level3)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT l1.name, l4.name
FROM level1 l1
JOIN level2 l2 ON l1.l2_id = l2.id
JOIN level3 l3 ON l2.l3_id = l3.id
JOIN level4 l4 ON l3.l4_id = l4.id
WHERE l2.name @@@ 'L2-B' AND l3.name @@@ 'L3-B'
ORDER BY l1.id
LIMIT 5;

SELECT l1.name, l4.name
FROM level1 l1
JOIN level2 l2 ON l1.l2_id = l2.id
JOIN level3 l3 ON l2.l3_id = l3.id
JOIN level4 l4 ON l3.l4_id = l4.id
WHERE l2.name @@@ 'L2-B' AND l3.name @@@ 'L3-B'
ORDER BY l1.id
LIMIT 5;

-- =============================================================================
-- TEST 36: Join on sorted keys (Both sides sorted on join key)
-- =============================================================================

DROP TABLE IF EXISTS sorted_t1 CASCADE;
DROP TABLE IF EXISTS sorted_t2 CASCADE;

CREATE TABLE sorted_t1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE sorted_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

INSERT INTO sorted_t1 SELECT i, 'val ' || i FROM generate_series(1, 1000) i;
INSERT INTO sorted_t2 SELECT i, (i % 1000) + 1, 'val ' || i FROM generate_series(1, 1000) i;

-- Indexes sorted by join key
-- t1 sorted by id
CREATE INDEX sorted_t1_idx ON sorted_t1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}');

-- t2 sorted by t1_id (the foreign key)
CREATE INDEX sorted_t2_idx ON sorted_t2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}');

ANALYZE sorted_t1;
ANALYZE sorted_t2;

-- Join on t1.id = t2.t1_id
-- Both are sorted on the join key (ASC NULLS FIRST)
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.val, t2.val
FROM sorted_t1 t1
JOIN sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

SELECT t1.val, t2.val
FROM sorted_t1 t1
JOIN sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

-- =============================================================================
-- TEST 36b: OFFSET + LIMIT on sorted join keys
-- PostgreSQL's limit_tuples includes the offset (5+10=15), so JoinScan passes
-- fetch=15 to DataFusion. The EXPLAIN should show SortExec: TopK(fetch=15)
-- wrapping StripOrderingExec. PostgreSQL's outer Limit applies the offset.
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.val, t2.val
FROM sorted_t1 t1
JOIN sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
OFFSET 5 LIMIT 10;

SELECT t1.val, t2.val
FROM sorted_t1 t1
JOIN sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
OFFSET 5 LIMIT 10;

-- =============================================================================
-- TEST 37: Multi-segment sorted join
-- =============================================================================

DROP TABLE IF EXISTS multi_seg_1 CASCADE;
DROP TABLE IF EXISTS multi_seg_2 CASCADE;

CREATE TABLE multi_seg_1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE multi_seg_2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

-- Force multiple segments using small mutable_segment_rows
CREATE INDEX multi_seg_1_idx ON multi_seg_1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}', mutable_segment_rows = 10);

CREATE INDEX multi_seg_2_idx ON multi_seg_2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}', mutable_segment_rows = 10);

-- Insert 100 rows, should create ~10 segments each
INSERT INTO multi_seg_1 SELECT i, 'val ' || i FROM generate_series(1, 100) i;
INSERT INTO multi_seg_2 SELECT i, (i % 100) + 1, 'val ' || i FROM generate_series(1, 100) i;

ANALYZE multi_seg_1;
ANALYZE multi_seg_2;

-- Verify SortMergeJoin is used with multi-segment indexes
-- MultiSegmentPlan exposes N partitions. SortMergeJoin should work.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.val, t2.val
FROM multi_seg_1 t1
JOIN multi_seg_2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

SELECT t1.val, t2.val
FROM multi_seg_1 t1
JOIN multi_seg_2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

-- =============================================================================
-- TEST 38: Recursive SortMergeJoin (3 tables sorted by t1.id)
-- =============================================================================

DROP TABLE IF EXISTS recursive_smj_1 CASCADE;
DROP TABLE IF EXISTS recursive_smj_2 CASCADE;
DROP TABLE IF EXISTS recursive_smj_3 CASCADE;

CREATE TABLE recursive_smj_1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE recursive_smj_2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);
CREATE TABLE recursive_smj_3 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

INSERT INTO recursive_smj_1 SELECT i, 'val ' || i FROM generate_series(1, 100) i;
INSERT INTO recursive_smj_2 SELECT i, i, 'val ' || i FROM generate_series(1, 100) i;
INSERT INTO recursive_smj_3 SELECT i, i, 'val ' || i FROM generate_series(1, 100) i;

-- Index for t1 sorted by id
CREATE INDEX recursive_smj_1_idx ON recursive_smj_1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}');

-- Index for t2 sorted by t1_id
CREATE INDEX recursive_smj_2_idx ON recursive_smj_2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}');

-- Index for t3 sorted by t1_id
CREATE INDEX recursive_smj_3_idx ON recursive_smj_3 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}');

ANALYZE recursive_smj_1;
ANALYZE recursive_smj_2;
ANALYZE recursive_smj_3;

-- Join 3 tables on t1.id
-- t1.id = t2.t1_id AND t1.id = t3.t1_id
-- All indexes are sorted by the respective join keys.
-- Should result in recursive SortMergeJoins without any SortExecs.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.val, t2.val, t3.val
FROM recursive_smj_1 t1
JOIN recursive_smj_2 t2 ON t1.id = t2.t1_id
JOIN recursive_smj_3 t3 ON t1.id = t3.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

SELECT t1.val, t2.val, t3.val
FROM recursive_smj_1 t1
JOIN recursive_smj_2 t2 ON t1.id = t2.t1_id
JOIN recursive_smj_3 t3 ON t1.id = t3.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

-- =============================================================================
-- TEST 39: TopK dynamic filter pushdown through SortMergeJoin
-- ORDER BY differs from join key => SortExec(TopK) stays in the plan.
-- Multiple segments ensure the scan produces multiple batches so TopK can
-- tighten its threshold between batches and the pre-filter actually prunes.
-- =============================================================================

DROP TABLE IF EXISTS dyn_filter_t1 CASCADE;
DROP TABLE IF EXISTS dyn_filter_t2 CASCADE;

CREATE TABLE dyn_filter_t1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE dyn_filter_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

-- Create indexes BEFORE inserting data so inserts go through the mutable
-- segment pathway, producing multiple segments (index-build on existing data
-- merges everything into one segment).
CREATE INDEX dyn_filter_t1_idx ON dyn_filter_t1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}', mutable_segment_rows = 10000);

CREATE INDEX dyn_filter_t2_idx ON dyn_filter_t2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}', mutable_segment_rows = 10000);

INSERT INTO dyn_filter_t1 SELECT i, 'val ' || i FROM generate_series(1, 20000) i;
INSERT INTO dyn_filter_t2 SELECT i, (i % 20000) + 1, 'val ' || i FROM generate_series(1, 20000) i;

ANALYZE dyn_filter_t1;
ANALYZE dyn_filter_t2;

-- EXPLAIN: check that dynamic_filters appear on the scan
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.val, t2.val
FROM dyn_filter_t1 t1
JOIN dyn_filter_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.val ASC
LIMIT 10;

-- Cap the scanner batch size so TopK can tighten its threshold between batches.
SET paradedb.dynamic_filter_batch_size = 8192;

-- EXPLAIN ANALYZE: rows_pruned should be > 0 with multiple segments
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM dyn_filter_t1 t1
JOIN dyn_filter_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.val ASC
LIMIT 10;

-- Verify results
SELECT t1.val, t2.val
FROM dyn_filter_t1 t1
JOIN dyn_filter_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.val ASC
LIMIT 10;

-- =============================================================================
-- TEST 39b: TopK dynamic filter does not prune NULLs
-- TopK emits "col IS NULL OR col < threshold". Rows with NULL in the ORDER BY
-- column must survive the pre-filter (nulls_pass=true) and be returned when
-- they belong in the top-K. Without nulls_pass, the pre-filter would
-- incorrectly discard NULLs.
--
-- Uses DESC NULLS FIRST so NULLs sort first and belong in the top-K result.
-- NULLs are placed at high IDs so they land in a later scan batch (after TopK
-- has already tightened its threshold from earlier batches). This ensures the
-- pre-filter is active when it encounters NULL values.
-- =============================================================================

DROP TABLE IF EXISTS null_val_t1 CASCADE;
DROP TABLE IF EXISTS null_val_t2 CASCADE;

CREATE TABLE null_val_t1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE null_val_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

CREATE INDEX null_val_t1_idx ON null_val_t1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}', mutable_segment_rows = 10000);

CREATE INDEX null_val_t2_idx ON null_val_t2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}', mutable_segment_rows = 10000);

-- 20K rows. Most have non-NULL val, but the last 10 (ids 19991-20000) are NULL.
-- With mutable_segment_rows=10000 the NULLs land in segment 2's later batch,
-- which is processed after TopK has updated its threshold.
INSERT INTO null_val_t1
  SELECT i,
         CASE WHEN i > 19990 THEN NULL ELSE 'val ' || i END
  FROM generate_series(1, 20000) i;
INSERT INTO null_val_t2
  SELECT i, (i % 20000) + 1, 'val ' || i
  FROM generate_series(1, 20000) i;

ANALYZE null_val_t1;
ANALYZE null_val_t2;

-- DESC NULLS FIRST: NULLs belong in the top 25.
-- The IS NULL OR pattern is decomposed into a PreFilter with nulls_pass=true.
-- EXPLAIN ANALYZE shows rows_pruned > 0 proving the pre-filter is active
-- (without the IS NULL OR decomposition, rows_pruned would be 0).
-- The NULLs in the result prove they survived the pre-filter correctly.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val' OR t1.val IS NULL
ORDER BY t1.val DESC NULLS FIRST
LIMIT 25;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val' OR t1.val IS NULL
ORDER BY t1.val DESC NULLS FIRST
LIMIT 25;

SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val' OR t1.val IS NULL
ORDER BY t1.val DESC NULLS FIRST
LIMIT 25;

-- =============================================================================
-- CLEANUP
-- =============================================================================

RESET paradedb.dynamic_filter_batch_size;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS level1 CASCADE;
DROP TABLE IF EXISTS level2 CASCADE;
DROP TABLE IF EXISTS level3 CASCADE;
DROP TABLE IF EXISTS level4 CASCADE;
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
DROP TABLE IF EXISTS uuid_orders CASCADE;
DROP TABLE IF EXISTS uuid_customers CASCADE;
DROP TABLE IF EXISTS numeric_transactions CASCADE;
DROP TABLE IF EXISTS numeric_accounts CASCADE;
DROP TABLE IF EXISTS large_items CASCADE;
DROP TABLE IF EXISTS large_categories CASCADE;
DROP TABLE IF EXISTS update_test_items CASCADE;
DROP TABLE IF EXISTS update_test_refs CASCADE;
DROP TABLE IF EXISTS qgen_products CASCADE;
DROP TABLE IF EXISTS qgen_users CASCADE;
DROP TABLE IF EXISTS tiny_products CASCADE;
DROP TABLE IF EXISTS tiny_refs CASCADE;
DROP TABLE IF EXISTS hint_test_products CASCADE;
DROP TABLE IF EXISTS hint_test_categories CASCADE;
DROP TABLE IF EXISTS sorted_t1 CASCADE;
DROP TABLE IF EXISTS sorted_t2 CASCADE;
DROP TABLE IF EXISTS dyn_filter_t1 CASCADE;
DROP TABLE IF EXISTS dyn_filter_t2 CASCADE;
DROP TABLE IF EXISTS null_val_t1 CASCADE;
DROP TABLE IF EXISTS null_val_t2 CASCADE;
DROP TABLE IF EXISTS multi_seg_1 CASCADE;
DROP TABLE IF EXISTS multi_seg_2 CASCADE;
DROP TABLE IF EXISTS recursive_smj_1 CASCADE;
DROP TABLE IF EXISTS recursive_smj_2 CASCADE;
DROP TABLE IF EXISTS recursive_smj_3 CASCADE;


RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
