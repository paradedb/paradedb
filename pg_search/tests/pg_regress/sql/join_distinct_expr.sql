-- Test for DISTINCT with expressions in JoinScan (Issue #4604)
-- Verifies that JoinScan activates and correctly deduplicates DISTINCT queries
-- with expression target lists via the PgExprUdf GROUP BY path.
--
-- Every test has EXPLAIN proving "ParadeDB Join Scan" with pdb_eval_expr_ in the plan,
-- followed by ON vs OFF result comparison.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS dex_products CASCADE;
DROP TABLE IF EXISTS dex_suppliers CASCADE;

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

INSERT INTO dex_suppliers (id, name, info, country) VALUES
(1, 'TechCorp', 'tech electronics supplier', 'USA'),
(2, NULL, 'unnamed electronics supplier', 'UK'),
(3, 'FastParts', 'fast delivery of electronics parts', 'Germany');

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

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: FuncExpr — DISTINCT upper(s.name)
-- =============================================================================

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
-- TEST 2: NullTest — DISTINCT s.name IS NULL
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT DISTINCT s.name IS NULL AS supplier_null, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

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
-- TEST 3: OpExpr — DISTINCT arithmetic expression
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT DISTINCT p.supplier_id * 10 + p.id AS combo_id, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

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
-- TEST 4: COALESCE — DISTINCT COALESCE(s.name, 'N/A')
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT DISTINCT COALESCE(s.name, 'N/A') AS safe_supplier, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

SELECT DISTINCT COALESCE(s.name, 'N/A') AS safe_supplier, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT COALESCE(s.name, 'N/A') AS safe_supplier, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 5: Cast + concat — DISTINCT s.name || '-' || p.supplier_id::text
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT DISTINCT s.name || '-' || p.supplier_id::text AS name_id, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

SELECT DISTINCT s.name || '-' || p.supplier_id::text AS name_id, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT s.name || '-' || p.supplier_id::text AS name_id, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 6: FuncExpr — DISTINCT length(s.name)
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT DISTINCT length(s.name) AS name_len, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

SELECT DISTINCT length(s.name) AS name_len, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT length(s.name) AS name_len, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 7: LIMIT 1 correctness — must return exactly 1 row, not 0
-- =============================================================================
-- Critical regression test: deduplication must happen BEFORE LIMIT.

EXPLAIN (COSTS OFF)
SELECT DISTINCT upper(s.name) IS NULL, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 1;

SELECT DISTINCT upper(s.name) IS NULL, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 1;

-- =============================================================================
-- TEST 8: Unsupported result type — graceful fallback to native PG
-- =============================================================================
-- to_jsonb returns JSONB which is not in is_arrow_convertible.
-- JoinScan should decline; PG handles it via native Hash Join / Unique.

EXPLAIN (COSTS OFF)
SELECT DISTINCT to_jsonb(s.name) AS supplier_json, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

-- Verify: no crash, no error, correct results via native PG.
SELECT DISTINCT to_jsonb(s.name) AS supplier_json, p.name
FROM dex_products p
         JOIN dex_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
ORDER BY p.name
    LIMIT 10;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS dex_products CASCADE;
DROP TABLE IF EXISTS dex_suppliers CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
