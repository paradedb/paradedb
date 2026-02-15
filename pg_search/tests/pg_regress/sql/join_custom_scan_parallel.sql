-- Test for the JoinScan Custom Scan planning with parallelism
-- This test verifies that the join custom scan is proposed and works correctly
-- when parallel workers are enabled and multiple segments exist.

-- Disable mutable segments to ensure multiple segments are created
SET paradedb.global_mutable_segment_rows = 0;

-- Enable parallel workers and force parallel plans even for small data
SET max_parallel_workers_per_gather = 2;
SET min_parallel_table_scan_size = 0;
SET min_parallel_index_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;

CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT,
    supplier_id INTEGER
);

-- Create BM25 indexes
-- We create them BEFORE inserting data so that each insert creates a new segment
-- due to paradedb.global_mutable_segment_rows = 0
CREATE INDEX suppliers_bm25_idx ON suppliers USING bm25 (id, name)
WITH (key_field = 'id');

CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, supplier_id)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}}');

-- Insert test data in separate batches to create multiple segments
INSERT INTO suppliers (id, name) VALUES (1, 'TechCorp');
INSERT INTO suppliers (id, name) VALUES (2, 'GlobalSupply');

INSERT INTO products (id, name, supplier_id) VALUES (1, 'Wireless Mouse', 1);
INSERT INTO products (id, name, supplier_id) VALUES (2, 'Wireless Keyboard', 1);
INSERT INTO products (id, name, supplier_id) VALUES (3, 'USB Cable', 2);
INSERT INTO products (id, name, supplier_id) VALUES (4, 'Monitor Stand', 2);
VACUUM FREEZE products;
VACUUM FREEZE suppliers;

-- =============================================================================
-- TEST 1: Parallel JoinScan with predicate on driving side
-- =============================================================================

SET paradedb.enable_join_custom_scan = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.name @@@ 'Wireless'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.name @@@ 'Wireless'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 2: Parallel JoinScan with predicate on build side
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE s.name @@@ 'TechCorp'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE s.name @@@ 'TechCorp'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 3: Parallel JoinScan with predicates on both sides
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.name @@@ 'Wireless' AND s.name @@@ 'TechCorp'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.name @@@ 'Wireless' AND s.name @@@ 'TechCorp'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 4: Parallel JoinScan with ORDER BY score
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name, paradedb.score(p.id)
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.name @@@ 'Wireless'
ORDER BY paradedb.score(p.id) DESC, p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name, paradedb.score(p.id)
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.name @@@ 'Wireless'
ORDER BY paradedb.score(p.id) DESC, p.id
LIMIT 10;

-- Cleanup
DROP TABLE products CASCADE;
DROP TABLE suppliers CASCADE;

RESET paradedb.global_mutable_segment_rows;
RESET max_parallel_workers_per_gather;
RESET min_parallel_table_scan_size;
RESET min_parallel_index_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;