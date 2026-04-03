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

-- Insert products — include NULLs and edge cases for name
INSERT INTO dex_products (id, name, description, supplier_id, category) VALUES
(101, 'Wireless Mouse', 'Ergonomic wireless mouse with Bluetooth', 1, 'Electronics'),
(102, 'USB Cable', 'High-speed USB cable for data wireless transfer', 2, 'Electronics'),
(103, 'Keyboard', 'Mechanical keyboard wireless enabled', 1, 'Electronics'),
(104, NULL, 'Unnamed wireless gadget for testing', 3, 'Office'),
(105, 'Headphones', 'Noise-canceling wireless headphones premium', 1, 'Electronics'),
(106, NULL, 'Another unnamed wireless product', 2, 'Office'),
(107, 'WIRELESS ROUTER', 'Enterprise wireless router', 1, 'Electronics'),
(108, 'tablet', 'Budget wireless tablet device', 2, 'Electronics'),
(109, '', 'Empty name wireless device', 1, 'Office');

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
-- 'wireless' matches all 9 products. 7 have names (incl empty string), 2 have NULL.

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
    LIMIT 20;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT upper(name) AS upper_name
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 3: Single-table OpExpr — SELECT DISTINCT id + 1
-- =============================================================================

SELECT DISTINCT id + 1 AS id_plus_one
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT id + 1 AS id_plus_one
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 4: Single-table LIMIT correctness — must return exactly 1 row
-- =============================================================================

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
    LIMIT 20;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT name || ' - ' || category AS combo
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST A1: IS NOT NULL
-- =============================================================================

SELECT DISTINCT name IS NOT NULL AS has_name
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT name IS NOT NULL AS has_name
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST A4: String length function
-- =============================================================================

SELECT DISTINCT length(name) AS name_len
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT length(name) AS name_len
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST A5: Arithmetic with multiple operators
-- =============================================================================

SELECT DISTINCT (id * 2 + supplier_id) AS computed
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT (id * 2 + supplier_id) AS computed
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST A6: COALESCE
-- =============================================================================

SELECT DISTINCT COALESCE(name, 'N/A') AS safe_name
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT COALESCE(name, 'N/A') AS safe_name
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST A7: lower()
-- =============================================================================

SELECT DISTINCT lower(name) AS lower_name
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT lower(name) AS lower_name
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST B1: Double function nesting — length(upper(name))
-- =============================================================================

SELECT DISTINCT length(upper(name)) AS upper_len
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT length(upper(name)) AS upper_len
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST B2: Expression mixing column types (text + int cast)
-- =============================================================================

SELECT DISTINCT name || '-' || supplier_id::text AS name_supplier
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT name || '-' || supplier_id::text AS name_supplier
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1
    LIMIT 20;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 7: Join — DISTINCT with expression, both tables have @@@ predicates
-- =============================================================================
-- DISTINCT upper(s.name) should collapse duplicates via PgExprUdf GROUP BY.
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
-- TEST C4: Join — arithmetic expression
-- =============================================================================

SELECT DISTINCT p.supplier_id * 10 + p.id AS combo_id, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT p.supplier_id * 10 + p.id AS combo_id, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST D1: Join — LIMIT 1 correctness
-- =============================================================================
-- Must return exactly 1 row, not 0.

SELECT DISTINCT upper(s.name) IS NULL, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 1;

-- =============================================================================
-- TEST D3: Empty result set (no matches)
-- =============================================================================

SELECT DISTINCT upper(name) AS upper_name
FROM dex_products
WHERE description @@@ 'nonexistentterm12345'
ORDER BY 1;

-- =============================================================================
-- TEST D4: Empty string vs NULL distinction
-- =============================================================================

SELECT DISTINCT name IS NULL AS is_null, name = '' AS is_empty
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1, 2;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT name IS NULL AS is_null, name = '' AS is_empty
FROM dex_products
WHERE description @@@ 'wireless'
ORDER BY 1, 2;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS dex_products CASCADE;
DROP TABLE IF EXISTS dex_suppliers CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
