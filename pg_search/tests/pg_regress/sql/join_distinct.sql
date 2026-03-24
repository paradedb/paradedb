-- Test for DISTINCT keyword support in JoinScan (Issue #4213)
-- This test verifies that JoinScan correctly handles DISTINCT queries:
-- 1. JoinScan activates for DISTINCT + ORDER BY + LIMIT queries
-- 2. Deduplication produces correct results matching native PG execution
-- 3. Fallback occurs when DISTINCT columns are not fast fields

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS dist_products CASCADE;
DROP TABLE IF EXISTS dist_suppliers CASCADE;
DROP TABLE IF EXISTS dist_categories CASCADE;

-- Create test tables
-- Suppliers: IDs 151-154
-- Products: IDs 201-210
-- Categories: IDs 301-303
--
-- Data is designed so that a product→supplier join produces 1:1 rows,
-- but a product→supplier join with MULTIPLE products per supplier creates
-- the duplicate supplier names that DISTINCT should collapse.

CREATE TABLE dist_suppliers
(
    id           INTEGER PRIMARY KEY,
    name         TEXT,
    contact_info TEXT,
    country      TEXT
);

CREATE TABLE dist_categories
(
    id          INTEGER PRIMARY KEY,
    name        TEXT,
    description TEXT
);

CREATE TABLE dist_products
(
    id          INTEGER PRIMARY KEY,
    name        TEXT,
    description TEXT,
    supplier_id INTEGER,
    category_id INTEGER,
    price       DECIMAL(10, 2)
);

-- Insert suppliers
INSERT INTO dist_suppliers (id, name, contact_info, country)
VALUES (151, 'TechCorp', 'contact@techcorp.com wireless technology', 'USA'),
       (152, 'GlobalSupply', 'info@globalsupply.com international shipping', 'UK'),
       (153, 'FastParts', 'sales@fastparts.com quick delivery', 'Germany'),
       (154, 'QualityFirst', 'quality@first.com premium products', 'Japan');

-- Insert categories
INSERT INTO dist_categories (id, name, description)
VALUES (301, 'Electronics', 'Electronic devices and components'),
       (302, 'Accessories', 'Computer accessories and peripherals'),
       (303, 'Office', 'Office supplies and equipment');

-- Insert products — multiple products per supplier to create duplicates on join
-- Supplier 151 (TechCorp) has 4 products: 201, 203, 206, 209
-- Supplier 152 (GlobalSupply) has 3 products: 202, 207, 210
-- Supplier 153 (FastParts) has 2 products: 204, 208
-- Supplier 154 (QualityFirst) has 1 product: 205
INSERT INTO dist_products (id, name, description, supplier_id, category_id, price)
VALUES (201, 'Wireless Mouse', 'Ergonomic wireless mouse with Bluetooth connectivity', 151, 302, 29.99),
       (202, 'USB Cable', 'High-speed USB-C cable for fast data transfer', 152, 302, 9.99),
       (203, 'Keyboard', 'Mechanical keyboard with RGB lighting wireless', 151, 301, 89.99),
       (204, 'Monitor Stand', 'Adjustable monitor stand for ergonomic setup', 153, 303, 49.99),
       (205, 'Webcam', 'HD webcam for video conferencing', 154, 301, 59.99),
       (206, 'Headphones', 'Wireless noise-canceling headphones with premium sound', 151, 301, 199.99),
       (207, 'Mouse Pad', 'Large gaming mouse pad with wireless charging', 152, 302, 39.69),
       (208, 'Cable Organizer', 'Desktop cable organizer for clean setup', 153, 303, 14.99),
       (209, 'Wireless Charger', 'Fast wireless charging pad for smartphones', 151, 301, 34.99),
       (210, 'USB Hub', 'Multi-port USB hub for data transfer connectivity', 152, 302, 24.99);

-- Create BM25 indexes with fast fields on join keys and ORDER BY columns
-- All columns that appear in DISTINCT target lists must be fast fields
CREATE INDEX dist_products_bm25_idx ON dist_products
    USING bm25 (id, name, description, supplier_id, category_id, price)
    WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true}}',
    numeric_fields = '{"supplier_id": {"fast": true}, "category_id": {"fast": true}, "price": {"fast": true}}'
    );

CREATE INDEX dist_suppliers_bm25_idx ON dist_suppliers
    USING bm25 (id, name, contact_info, country)
    WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true}}'
    );

CREATE INDEX dist_categories_bm25_idx ON dist_categories
    USING bm25 (id, name, description)
    WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true}}'
    );

-- Enable JoinScan
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: Basic DISTINCT with JoinScan activation
-- =============================================================================
-- Verify that JoinScan is chosen for a DISTINCT + ORDER BY + LIMIT query.
-- Without DISTINCT support, JoinScan would bail out because PG sets
-- limit_tuples = -1.0 when DISTINCT is present.

EXPLAIN
(COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 10;

-- =============================================================================
-- TEST 2: DISTINCT actually deduplicates rows
-- =============================================================================
-- 'wireless' matches products 201, 203, 206, 207, 209.
-- Products 201, 203, 206, 209 share supplier_id=151 (TechCorp),
-- and 207 has supplier_id=152 (GlobalSupply).
-- Without DISTINCT: s.name shows TechCorp 4 times + GlobalSupply once = 5 rows.
-- With DISTINCT on s.name: collapses to 2 rows (GlobalSupply, TechCorp).

-- Non-DISTINCT: expect 5 rows with TechCorp repeated
EXPLAIN
SELECT s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY s.name
    LIMIT 20;

SELECT s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY s.name
    LIMIT 20;

-- DISTINCT: expect exactly 2 rows (GlobalSupply, TechCorp)
SELECT DISTINCT s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY s.name
    LIMIT 10;

-- =============================================================================
-- TEST 3: Correctness — compare JoinScan vs native PostgreSQL
-- =============================================================================
-- Run the same DISTINCT query with JoinScan enabled and disabled.
-- Results must be identical.

-- JoinScan enabled (should use ParadeDB Join Scan)
SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 10;

-- JoinScan disabled (native PostgreSQL execution)
SET paradedb.enable_join_custom_scan = off;

SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 10;

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 4: DISTINCT with score column
-- =============================================================================
-- paradedb.score() in a DISTINCT query. Score values may differ across rows
-- in the same "output group", so DISTINCT on (name, score) may or may not
-- collapse rows depending on score precision.

EXPLAIN
(COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT p.name, paradedb.score(p.id) AS score
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY paradedb.score(p.id) DESC, p.name
    LIMIT 10;

SELECT DISTINCT p.name, paradedb.score(p.id) AS score
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY paradedb.score(p.id) DESC, p.name
    LIMIT 10;

-- =============================================================================
-- TEST 5: DISTINCT without ORDER BY
-- =============================================================================
-- DISTINCT without explicit ORDER BY. PostgreSQL may still add pathkeys
-- for the Unique node, but there's no user-facing sort requirement.
-- JoinScan should still handle this correctly.
-- ORDER BY s.name added here only to make output deterministic for test stability.

SELECT DISTINCT s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY s.name
    LIMIT 10;

-- =============================================================================
-- TEST 6: Multi-table join (3 tables) with DISTINCT
-- =============================================================================
-- Star schema: products joins suppliers AND categories.
-- DISTINCT should collapse rows where the same (product, supplier, category)
-- triple appears multiple times.

EXPLAIN
(COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT p.name AS product, s.name AS supplier, c.name AS category
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
         JOIN dist_categories c ON p.category_id = c.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 10;

SELECT DISTINCT p.name AS product, s.name AS supplier, c.name AS category
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
         JOIN dist_categories c ON p.category_id = c.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 10;

-- Compare with native PG
SET paradedb.enable_join_custom_scan = off;

SELECT DISTINCT p.name AS product, s.name AS supplier, c.name AS category
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
         JOIN dist_categories c ON p.category_id = c.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 10;

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 7: DISTINCT with duplicate columns across tables
-- =============================================================================
-- Both products and suppliers have a 'name' column. Selecting both with
-- DISTINCT should deduplicate on the pair (p.name, s.name), not confuse
-- the two 'name' columns.

SELECT DISTINCT p.name AS product_name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 10;

-- =============================================================================
-- TEST 8: Single-sort guarantee — verify plan shape
-- =============================================================================
-- The DISTINCT pipeline must be: GROUP BY → Sort → Limit (single sort).
-- A double-sort (Sort → GROUP BY → Sort → Limit) would mean the first sort
-- is wasted work because GROUP BY destroys ordering. Verify:
-- 1. AggregateExec appears in the physical plan (the GROUP BY)
-- 2. SortExec appears exactly once, AFTER AggregateExec (not before)
-- 3. No SortExec appears below AggregateExec
-- Uses ORDER BY s.name (different from TEST 1's ORDER BY p.name) to confirm
-- the sort column choice doesn't affect the single-sort guarantee.

EXPLAIN
(COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY s.name
    LIMIT 10;

-- =============================================================================
-- TEST 9: Non-DISTINCT queries still use SegmentedTopKExec
-- =============================================================================
-- Ensure that non-DISTINCT queries are not affected by the DISTINCT changes.
-- Should show SegmentedTopKExec in the physical plan, not AggregateExec.

EXPLAIN
(COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 10;

-- =============================================================================
-- TEST 9b: WHERE clause anchoring for LateMaterializeNode
-- =============================================================================
-- Verify that when a deferred column (e.g. s.name) is used in a WHERE clause,
-- LateMaterializeNode correctly anchors below the Filter node so it can be evaluated.

EXPLAIN
(COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.name = 'TechCorp'
ORDER BY p.name
    LIMIT 10;

-- =============================================================================
-- TEST 10: Fallback — DISTINCT with non-fast-field column
-- =============================================================================
-- 'description' is indexed but NOT a fast field (it's a text field without
-- fast:true). If DISTINCT includes description, JoinScan should fall back
-- because deduplication requires fast field access for all DISTINCT columns.
-- Expect a planner warning about DISTINCT columns not being fast fields.

EXPLAIN
(COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT p.name, p.description
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 10;

-- =============================================================================
-- TEST 11: Fallback — COUNT(*) with LIMIT
-- =============================================================================
-- Aggregate queries (COUNT, SUM, etc.) with LIMIT should NOT use JoinScan.
-- This verifies that aggregates don't interfere with the DISTINCT path.

EXPLAIN
(COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*)
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
    LIMIT 10;

-- =============================================================================
-- TEST 12: Aggregate correctness — COUNT not capped by LIMIT
-- =============================================================================
-- Verify that COUNT(*) returns the true row count (5), not the LIMIT value (3).
-- This was a bug where JoinScan pushed LIMIT into DataFusion before the
-- aggregate saw all rows, causing COUNT to return min(true_count, limit).

SELECT COUNT(*)
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
    LIMIT 3;

-- =============================================================================
-- TEST 13: Fallback — DISTINCT without LIMIT
-- =============================================================================
-- DISTINCT without a LIMIT clause. JoinScan requires a LIMIT to activate,
-- so this should fall back to native PG execution with the standard
-- "JoinScan not used: query must have a LIMIT clause" warning.

EXPLAIN
(COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name;

-- =============================================================================
-- TEST 14: DISTINCT with search predicates on both sides
-- =============================================================================
-- Both tables have @@@ predicates. DISTINCT should still work correctly.

SELECT DISTINCT p.name AS product, s.name AS supplier
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.contact_info @@@ 'technology'
ORDER BY p.name
    LIMIT 10;

-- Compare with native PG
SET paradedb.enable_join_custom_scan = off;

SELECT DISTINCT p.name AS product, s.name AS supplier
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.contact_info @@@ 'technology'
ORDER BY p.name
    LIMIT 10;

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 15: DISTINCT with LIMIT smaller than distinct count
-- =============================================================================
-- Verify that LIMIT correctly caps the output AFTER deduplication.
-- 'wireless' matches 5 products. DISTINCT (p.name, s.name) should produce
-- 5 unique rows. LIMIT 3 should return only the first 3 after sort.

SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 3;

-- =============================================================================
-- TEST 16: DISTINCT with multiple OR conditions
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT
    p.id,
    p.name,
    paradedb.score(p.id) as score
FROM dist_products p
JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE
    s.country @@@ 'USA'
    AND (
        p.name @@@ 'Wireless'
        OR
        s.name @@@ 'Wireless'
    )
ORDER BY
    score DESC
LIMIT 10;

SELECT DISTINCT
    p.id,
    p.name,
    paradedb.score(p.id) as score
FROM dist_products p
JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE
    s.country @@@ 'USA'
    AND (
        p.name @@@ 'Wireless'
        OR
        s.name @@@ 'Wireless'
    )
ORDER BY
    score DESC
LIMIT 10;

-- =============================================================================
-- TEST 17: DISTINCT with LIMIT + OFFSET
-- =============================================================================
-- Verify that OFFSET correctly skips rows AFTER deduplication and sorting.
-- 'wireless' matches 5 distinct (p.name, s.name) pairs ordered by p.name:
--   1. Headphones    | TechCorp
--   2. Keyboard      | TechCorp
--   3. Mouse Pad     | GlobalSupply
--   4. Wireless Charger | TechCorp
--   5. Wireless Mouse   | TechCorp
-- LIMIT 3 OFFSET 0 should return rows 1-3.
-- LIMIT 3 OFFSET 2 should return rows 3-5.

-- Baseline: OFFSET 0 (same as no OFFSET)
SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 3 OFFSET 0;
-- Expected: Headphones, Keyboard, Mouse Pad

-- Mid-page: OFFSET 2 should skip the first 2 rows
SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 3 OFFSET 2;
-- Expected: Mouse Pad, Wireless Charger, Wireless Mouse

-- Last page: OFFSET 4 should return only 1 row (past the end of 5 rows)
SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 3 OFFSET 4;
-- Expected: Wireless Mouse (only 1 row since 5 - 4 = 1)

-- OFFSET beyond result set should return empty
SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 3 OFFSET 10;
-- Expected: 0 rows

-- Correctness: compare JoinScan vs native PG for OFFSET 2
SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 3 OFFSET 2;
-- Expected: same as JoinScan result above
SET paradedb.enable_join_custom_scan = on;

-- Verify EXPLAIN shows Offset in the custom scan output
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT p.name, s.name AS supplier_name
FROM dist_products p
         JOIN dist_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.name
    LIMIT 3 OFFSET 2;
-- Expected: Offset: 2 appears in the Custom Scan (ParadeDB Join Scan) node

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS dist_products CASCADE;
DROP TABLE IF EXISTS dist_suppliers CASCADE;
DROP TABLE IF EXISTS dist_categories CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
