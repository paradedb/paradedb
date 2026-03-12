-- Test for the JoinScan Custom Scan planning
-- Basic functionality: limits, fallback, and basic queries.

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
-- TEST 6: Non-equijoin conditions (arbitrary join expressions)
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
-- TEST 7: LIMIT without ORDER BY vs with ORDER BY
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
-- TEST 8: Cross join (no equi-join keys) - JoinScan NOT proposed
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
-- TEST 9: Mixed-case column names (regression test for quoting issues)
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
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;
DROP TABLE IF EXISTS colors CASCADE;
DROP TABLE IF EXISTS sizes CASCADE;
DROP TABLE IF EXISTS "MixedCaseTable" CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
