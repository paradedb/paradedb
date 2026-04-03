-- Test for DISTINCT with expressions in JoinScan (Issue #4604)
-- This test verifies that JoinScan correctly handles DISTINCT queries with
-- expression target lists: NullTest, FuncExpr, OpExpr, nested, and multi-column.
--
-- Each test runs with JoinScan ON and OFF and compares results.

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS dex_products CASCADE;
DROP TABLE IF EXISTS dex_suppliers CASCADE;

-- Products: single-table tests use this table alone.
-- Some products have NULL names to test NullTest expressions.
CREATE TABLE dex_products
(
    id          INTEGER PRIMARY KEY,
    name        TEXT,
    description TEXT,
    category    TEXT,
    supplier_id INTEGER
);

CREATE TABLE dex_suppliers
(
    id      INTEGER PRIMARY KEY,
    name    TEXT,
    info    TEXT,
    country TEXT
);

-- Insert products — include NULLs for name to test IS NULL
INSERT INTO dex_products (id, name, description, supplier_id, category) VALUES
(101, 'Wireless Mouse', 'Ergonomic wireless mouse with Bluetooth', 1, 'Electronics'),
(102, 'USB Cable', 'High-speed USB cable for data wireless transfer', 2, 'Electronics'),
(103, 'Keyboard', 'Mechanical keyboard wireless enabled', 1, 'Electronics'),
(104, NULL, 'Unnamed wireless gadget for testing', 3, 'Office'),
(105, 'Headphones', 'Noise-canceling wireless headphones premium', 1, 'Electronics'),
(106, NULL, 'Another unnamed wireless product', 2, 'Office');

-- Insert suppliers
INSERT INTO dex_suppliers (id, name, info, country) VALUES
(1, 'TechCorp', 'tech electronics supplier', 'USA'),
(2, NULL, 'unnamed electronics supplier', 'UK'),
(3, 'FastParts', 'fast delivery of electronics parts', 'Germany');

-- Create BM25 indexes with fast fields
CREATE INDEX dex_products_bm25 ON dex_products
    USING bm25 (id, name, description, category, supplier_id)
    WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true}, "category": {"fast": true}}',
    numeric_fields = '{"supplier_id": {"fast": true}}'
    );

CREATE INDEX dex_suppliers_bm25 ON dex_suppliers
    USING bm25 (id, name, info, country)
    WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true}}'
    );

-- Enable JoinScan
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: Single-table NullTest — SELECT DISTINCT name IS NULL
-- =============================================================================
-- 'wireless' matches all 6 products (101-106). 4 have names, 2 have NULL.
-- DISTINCT (name IS NULL) should yield {true, false}.

EXPLAIN (COSTS OFF)
SELECT DISTINCT name IS NULL AS is_null
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;

SELECT DISTINCT name IS NULL AS is_null
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT name IS NULL AS is_null
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 2: Single-table FuncExpr — SELECT DISTINCT upper(name)
-- =============================================================================

SELECT DISTINCT upper(name) AS upper_name
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT upper(name) AS upper_name
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 3: Single-table OpExpr — SELECT DISTINCT id + 1
-- =============================================================================

SELECT DISTINCT id + 1 AS id_plus_one
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT id + 1 AS id_plus_one
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 4: Single-table LIMIT correctness — must return exactly 1 row
-- =============================================================================
-- name IS NULL has at most 2 distinct values. LIMIT 1 must return 1, not 0.
-- This validates that deduplication happens BEFORE LIMIT.

SELECT DISTINCT name IS NULL
FROM dex_products
WHERE description @@@ 'wireless'
    LIMIT 1;

-- =============================================================================
-- TEST 5: Single-table nested expression — upper(name) IS NULL
-- =============================================================================

SELECT DISTINCT upper(name) IS NULL AS upper_is_null
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT upper(name) IS NULL AS upper_is_null
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 6: Single-table multi-column expression — name || ' - ' || category
-- =============================================================================

SELECT DISTINCT name || ' - ' || category AS combo
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT name || ' - ' || category AS combo
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 7: Join — DISTINCT with expression, both tables have @@@ predicates
-- =============================================================================
-- 'wireless' matches products 101-106 (all have 'wireless' in description).
-- 'electronics' matches suppliers 1, 2, 3 (all have 'electronics' in info).
-- After join on supplier_id: products join their suppliers.
-- DISTINCT upper(s.name) should collapse duplicates.
-- ORDER BY uses a plain fast-field column so JoinScan can validate ORDER BY.

EXPLAIN (COSTS OFF)
SELECT DISTINCT upper(s.name) AS upper_supplier, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

SELECT DISTINCT upper(s.name) AS upper_supplier, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT upper(s.name) AS upper_supplier, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 8: Join — DISTINCT s.name IS NULL with both @@@ predicates
-- =============================================================================

SELECT DISTINCT s.name IS NULL AS supplier_null, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT s.name IS NULL AS supplier_null, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS dex_products CASCADE;
DROP TABLE IF EXISTS dex_suppliers CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
