-- Test for the JoinScan Custom Scan planning
-- Joins involving 3 or more tables (star and chain schemas) and stale CTID handling.

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

-- Make sure the GUC is enabled
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: Multi-table joins (3 tables) - includes UPDATE that moves product ctids
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
-- TEST 2: OR across tables (without LIMIT) - AFTER UPDATE moved product ctids
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
-- TEST 3: OR across tables WITH LIMIT - AFTER UPDATE moved product ctids
-- =============================================================================
-- OR across tables WITH LIMIT - uses JoinScan.
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
-- TEST 4: Multi-table join - Star Schema (3 tables)
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
-- TEST 5: Multi-table join - Chain Schema (4 tables)
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
-- TEST 6: Chain Schema - Mixed Predicates
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
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS level1 CASCADE;
DROP TABLE IF EXISTS level2 CASCADE;
DROP TABLE IF EXISTS level3 CASCADE;
DROP TABLE IF EXISTS level4 CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
