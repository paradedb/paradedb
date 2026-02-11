-- Test for TopK dynamic filter pushdown through DataFusion
-- This test verifies that SortExec(TopK) propagates a DynamicFilterPhysicalExpr
-- down to PgSearchScan, enabling row pruning at the scan level for ORDER BY ... LIMIT queries.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;

CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    supplier_id INTEGER,
    price NUMERIC(10,2)
);

CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY,
    name TEXT,
    contact_info TEXT,
    country TEXT
);

INSERT INTO suppliers (id, name, contact_info, country) VALUES
(101, 'TechCorp', 'contact@techcorp.com wireless technology', 'USA'),
(102, 'GlobalSupply', 'info@globalsupply.com international shipping', 'UK'),
(103, 'FastParts', 'sales@fastparts.com quick delivery', 'Germany'),
(104, 'QualityFirst', 'quality@first.com premium products', 'Japan');

INSERT INTO products (id, name, description, supplier_id, price) VALUES
(1, 'Wireless Mouse', 'Ergonomic wireless mouse with Bluetooth connectivity', 101, 29.99),
(2, 'USB Cable', 'High-speed USB-C cable for fast data transfer', 102, 9.99),
(3, 'Keyboard', 'Mechanical keyboard with RGB lighting', 101, 89.99),
(4, 'Monitor Stand', 'Adjustable monitor stand for ergonomic setup', 103, 49.99),
(5, 'Webcam', 'HD webcam for video conferencing', 104, 59.99),
(6, 'Headphones', 'Wireless noise-canceling headphones with premium sound', 101, 199.99),
(7, 'Mouse Pad', 'Large gaming mouse pad with wireless charging', 102, 39.69),
(8, 'Cable Organizer', 'Desktop cable organizer for clean setup', 103, 14.99),
(9, 'Docking Station', 'USB-C docking station with wireless display', 104, 149.99),
(10, 'Power Strip', 'Smart power strip with wireless control', 102, 24.99);

CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, description, supplier_id, price)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}, "price": {"fast": true}}');
CREATE INDEX suppliers_bm25_idx ON suppliers USING bm25 (id, name, contact_info, country)
WITH (key_field = 'id');

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: EXPLAIN shows dynamic_filter=true on PgSearchScan with ORDER BY + LIMIT
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 3;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id
LIMIT 3;

-- =============================================================================
-- TEST 2: ORDER BY DESC + LIMIT
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id DESC
LIMIT 2;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id DESC
LIMIT 2;

-- =============================================================================
-- TEST 3: ORDER BY numeric (price) + LIMIT
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, p.price, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.price ASC
LIMIT 2;

SELECT p.id, p.name, p.price, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.price ASC
LIMIT 2;

-- =============================================================================
-- TEST 4: Both sides have search predicates with ORDER BY + LIMIT
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.contact_info @@@ 'technology'
ORDER BY p.id
LIMIT 5;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.contact_info @@@ 'technology'
ORDER BY p.id
LIMIT 5;

-- =============================================================================
-- TEST 5: Without LIMIT - no dynamic filter should appear
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
ORDER BY p.id;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE products CASCADE;
DROP TABLE suppliers CASCADE;
