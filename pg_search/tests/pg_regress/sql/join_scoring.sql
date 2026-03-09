-- Test for the JoinScan Custom Scan planning
-- Score propagation: paradedb.score() on driving and build sides.

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
-- TEST 1: ORDER BY pdb.score() - Single Feature join pattern
-- =============================================================================

-- This is the canonical "Single Feature" join pattern from the Top K spec.
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
-- TEST 2: SELECT paradedb.score() WITHOUT ORDER BY score
-- =============================================================================

-- This test verifies that paradedb.score() works correctly in SELECT
-- even when ORDER BY is on a different column (not score).
-- This is an edge case where scores must still be computed for output.
-- It works because Top K executor (used when LIMIT is present) always
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
-- TEST 3: paradedb.score() from build side (not driving side)
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
-- TEST 4: Both driving side AND build side scores in the same query
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
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
