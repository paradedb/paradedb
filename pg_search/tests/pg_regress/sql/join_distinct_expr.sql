-- Test for DISTINCT with expressions in JoinScan (Issue #4604)
-- This test verifies that JoinScan correctly handles DISTINCT queries with
-- expression target lists: NullTest, FuncExpr, Cast, OpExpr, nested, multi-column.

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS expr_products CASCADE;
DROP TABLE IF EXISTS expr_suppliers CASCADE;

CREATE TABLE expr_suppliers
(
    id      INTEGER PRIMARY KEY,
    name    TEXT,
    info    TEXT,
    country TEXT
);

CREATE TABLE expr_products
(
    id          INTEGER PRIMARY KEY,
    name        TEXT,
    description TEXT,
    supplier_id INTEGER REFERENCES expr_suppliers(id),
    category    TEXT,
    price       DECIMAL(10, 2)
);

-- Insert suppliers (some with NULL names for NullTest)
INSERT INTO expr_suppliers (id, name, info, country) VALUES
(1, 'TechCorp', 'tech supplier of electronics', 'USA'),
(2, NULL, 'unnamed electronics supplier', 'UK'),
(3, 'FastParts', 'fast delivery of parts', 'Germany'),
(4, NULL, 'another unnamed supplier of electronics', 'Japan');

-- Insert products with duplicates to test deduplication
INSERT INTO expr_products (id, name, description, supplier_id, category, price) VALUES
(101, 'Wireless Mouse', 'Ergonomic wireless mouse with Bluetooth', 1, 'Electronics', 29.99),
(102, 'USB Cable', 'High-speed USB cable for data transfer', 2, 'Electronics', 9.99),
(103, 'Keyboard', 'Mechanical keyboard wireless', 1, 'Electronics', 89.99),
(104, 'Monitor Stand', 'Adjustable monitor stand', 3, 'Office', 49.99),
(105, 'Webcam', 'HD webcam for conferencing', 4, 'Electronics', 59.99),
(106, 'Headphones', 'Noise-canceling wireless headphones', 1, 'Electronics', 199.99),
(107, 'Mouse Pad', 'Large gaming mouse pad', 2, 'Electronics', 39.69),
(108, 'Cable Organizer', 'Desktop cable organizer', 3, 'Office', 14.99);

-- Create BM25 indexes with fast fields
CREATE INDEX expr_products_bm25 ON expr_products
    USING bm25 (id, name, description, supplier_id, category, price)
    WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true}, "category": {"fast": true}}',
    numeric_fields = '{"supplier_id": {"fast": true}, "price": {"fast": true}}'
    );

CREATE INDEX expr_suppliers_bm25 ON expr_suppliers
    USING bm25 (id, name, info, country)
    WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true}, "country": {"fast": true}}'
    );

-- Enable JoinScan
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: NullTest — SELECT DISTINCT name IS NULL
-- =============================================================================
-- Suppliers matched via electronics products: TechCorp (id=1), NULL (id=2),
-- FastParts (id=3, not electronics), NULL (id=4).
-- name IS NULL should yield {true, false}.

-- JoinScan result
SELECT DISTINCT s.name IS NULL AS is_null
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;

-- Native PG result for comparison
SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT s.name IS NULL AS is_null
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 2: FuncExpr — SELECT DISTINCT upper(name)
-- =============================================================================

-- JoinScan result
SELECT DISTINCT upper(s.name) AS upper_name
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;

-- Native PG result for comparison
SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT upper(s.name) AS upper_name
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 3: Cast — SELECT DISTINCT price::text
-- =============================================================================

-- JoinScan result
SELECT DISTINCT p.price::text AS price_text
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;

-- Native PG result for comparison
SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT p.price::text AS price_text
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 4: OpExpr — SELECT DISTINCT id + 1
-- =============================================================================

-- JoinScan result
SELECT DISTINCT p.id + 1 AS id_plus_one
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;

-- Native PG result for comparison
SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT p.id + 1 AS id_plus_one
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 5: LIMIT correctness — must return exactly 1 row, not 0
-- =============================================================================
-- This is the critical test for why Option 3 (post-DF evaluation) fails.
-- name IS NULL has at most 2 distinct values. LIMIT 1 must return 1.

SELECT DISTINCT s.name IS NULL
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
LIMIT 1;

-- =============================================================================
-- TEST 6: Nested expression — SELECT DISTINCT upper(name) IS NULL
-- =============================================================================

-- JoinScan result
SELECT DISTINCT upper(s.name) IS NULL AS upper_is_null
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;

-- Native PG result for comparison
SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT upper(s.name) IS NULL AS upper_is_null
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 7: Multi-column expression — SELECT DISTINCT name || ' - ' || category
-- =============================================================================

-- JoinScan result
SELECT DISTINCT p.name || ' - ' || p.category AS combo
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;

-- Native PG result for comparison
SET paradedb.enable_join_custom_scan = off;
SELECT DISTINCT p.name || ' - ' || p.category AS combo
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 8: Verify EXPLAIN shows Custom Scan for expression DISTINCT
-- =============================================================================

EXPLAIN (COSTS OFF)
SELECT DISTINCT s.name IS NULL
FROM expr_products p
    JOIN expr_suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY 1
LIMIT 10;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS expr_products CASCADE;
DROP TABLE IF EXISTS expr_suppliers CASCADE;
