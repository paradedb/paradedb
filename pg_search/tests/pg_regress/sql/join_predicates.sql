-- Test for the JoinScan Custom Scan planning
-- Complex boolean expressions (AND/OR/NOT), side-level vs. join-level predicates, and multi-table fast fields.

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
-- TEST 1: Both sides have search predicates
-- =============================================================================

-- When both sides have @@@ predicates with LIMIT, JoinScan should be proposed
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.contact_info @@@ 'technology'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' AND s.contact_info @@@ 'technology'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 2: Both side-level AND join-level predicates combined
-- =============================================================================
-- This test shows a query where both sides have side-level predicates AND
-- there's a join-level predicate spanning both tables.
-- Side-level inner: p.description @@@ 'wireless' matches 201,206,207
-- Side-level outer: s.contact_info @@@ 'technology' matches 151
-- Join candidates after side filters: (201,151), (206,151)
-- Join-level: p.name @@@ 'headphones' OR s.name @@@ 'TechCorp'
--   - p.name @@@ 'headphones': matches 206
--   - s.name @@@ 'TechCorp': matches 151
-- Since supplier 151 matches 'TechCorp', both (201,151) and (206,151) pass
-- Expected: 2 rows (201 and 206)

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND s.contact_info @@@ 'technology'
  AND (p.name @@@ 'headphones' OR s.name @@@ 'TechCorp')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND s.contact_info @@@ 'technology'
  AND (p.name @@@ 'headphones' OR s.name @@@ 'TechCorp')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 3: Aggregate Score pattern - OR across tables (without LIMIT)
-- =============================================================================

-- NOTE: This case should propose JoinScan even WITHOUT LIMIT because
-- there's a join-level search predicate (OR spanning both relations).
-- This is the "Aggregate Score" pattern from the spec (planned for M3).
-- Currently falls back to Hash Join since M1 only handles LIMIT cases.
-- EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
-- SELECT p.id, p.name, s.name AS supplier_name
-- FROM products p
-- JOIN suppliers s ON p.supplier_id = s.id
-- WHERE p.description @@@ 'wireless' OR s.contact_info @@@ 'wireless';

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless' OR s.contact_info @@@ 'wireless'
ORDER BY p.id;

-- =============================================================================
-- TEST 4: OR across tables WITH LIMIT
-- =============================================================================

-- Same as TEST 3 but with LIMIT.
-- JoinScan IS proposed for join-level predicates (OR across tables).
-- The OR condition means a row passes if EITHER the product description
-- contains 'wireless' OR the supplier contact_info contains 'wireless'.
-- EXPECTED: 4 rows matching the OR condition (same as TEST 3).
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
-- TEST 5: Complex join-level predicate with NOT and OR
-- =============================================================================

-- Complex condition: (p.description @@@ 'wireless' AND NOT p.description @@@ 'mouse') OR s.contact_info @@@ 'shipping'
-- This tests:
-- 1. Negation within a search predicate
-- 2. OR combining predicates across tables
-- 3. AND within a single table's predicate
-- EXPECTED: Products matching 'wireless' but NOT 'mouse' in description,
--           OR any product joined to a supplier with 'shipping' in contact_info
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'wireless' AND NOT p.description @@@ 'mouse') OR s.contact_info @@@ 'shipping'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'wireless' AND NOT p.description @@@ 'mouse') OR s.contact_info @@@ 'shipping'
ORDER BY p.id
LIMIT 10;

-- Another complex pattern: NOT (p.description @@@ 'cable' OR p.description @@@ 'stand')
-- Products that do NOT contain 'cable' AND do NOT contain 'stand'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE NOT (p.description @@@ 'cable' OR p.description @@@ 'stand')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE NOT (p.description @@@ 'cable' OR p.description @@@ 'stand')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 6: Deeply nested boolean expressions
-- =============================================================================

-- Deeply nested: (p_cond1 OR (p_cond2 OR (s_cond AND NOT p_cond3)))
-- This tests the recursive expression tree building
-- p_cond1: p.description @@@ 'keyboard'
-- p_cond2: p.description @@@ 'headphones'  
-- s_cond: s.contact_info @@@ 'shipping'
-- p_cond3: p.description @@@ 'wireless'
-- EXPECTED: Products with 'keyboard' OR 'headphones' OR (supplier has 'shipping' AND product NOT 'wireless')
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'keyboard' OR (p.description @@@ 'headphones' OR (s.contact_info @@@ 'shipping' AND NOT p.description @@@ 'wireless'))
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'keyboard' OR (p.description @@@ 'headphones' OR (s.contact_info @@@ 'shipping' AND NOT p.description @@@ 'wireless'))
ORDER BY p.id
LIMIT 10;

-- AND of multiple single-table predicates combined with OR across tables
-- ((p.description @@@ 'wireless' AND p.description @@@ 'mouse') OR (s.contact_info @@@ 'shipping' AND s.country @@@ 'UK'))
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'wireless' AND p.description @@@ 'mouse') OR (s.contact_info @@@ 'shipping' AND s.country @@@ 'UK')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'wireless' AND p.description @@@ 'mouse') OR (s.contact_info @@@ 'shipping' AND s.country @@@ 'UK')
ORDER BY p.id
LIMIT 10;

-- Triple-nested NOT: NOT (NOT (NOT p.description @@@ 'cable'))
-- Equivalent to: NOT p.description @@@ 'cable'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE NOT (NOT (NOT p.description @@@ 'cable'))
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE NOT (NOT (NOT p.description @@@ 'cable'))
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 7: Qgen-style setup - Index before data with NOT operator
-- =============================================================================
-- This test replicates the qgen test setup which revealed a bug:
-- 1. Create index BEFORE inserting data (creates multiple segments)
-- 2. Use NOT operator in WHERE clause
-- 3. Join on non-PK column
-- 4. Larger dataset (100 rows)
-- Error was: "could not read blocks 65536..65536: read only 0 of 8192 bytes"

DROP TABLE IF EXISTS qgen_products CASCADE;
DROP TABLE IF EXISTS qgen_users CASCADE;

-- Use SERIAL8 (bigint) like the qgen test to verify the fix
CREATE TABLE qgen_users (
    id SERIAL8 PRIMARY KEY,
    uuid UUID,
    name TEXT,
    color TEXT,
    age INTEGER,
    quantity INTEGER,
    price NUMERIC(10,2),
    rating INTEGER
);

CREATE TABLE qgen_products (
    id SERIAL8 PRIMARY KEY,
    uuid UUID,
    name TEXT,
    color TEXT,
    age INTEGER,
    quantity INTEGER,
    price NUMERIC(10,2),
    rating INTEGER
);

-- Create index BEFORE inserting data (this is the key difference from other tests)
-- This causes multiple segments to be created as data is inserted
-- Note: age must be a fast field for the join key
CREATE INDEX qgen_users_bm25_idx ON qgen_users USING bm25 (id, name, age) WITH (
    key_field = 'id',
    text_fields = '{ "name": { "tokenizer": { "type": "keyword" }, "fast": true } }',
    numeric_fields = '{ "age": { "fast": true } }'
);

CREATE INDEX qgen_products_bm25_idx ON qgen_products USING bm25 (id, name, age) WITH (
    key_field = 'id',
    text_fields = '{ "name": { "tokenizer": { "type": "keyword" }, "fast": true } }',
    numeric_fields = '{ "age": { "fast": true } }'
);

-- Insert sample value first
INSERT INTO qgen_users (uuid, name, color, age, quantity, price, rating) 
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'bob', 'blue', 20, 7, 99.99, 4);

INSERT INTO qgen_products (uuid, name, color, age, quantity, price, rating) 
VALUES ('550e8400-e29b-41d4-a716-446655440000', 'bob', 'blue', 20, 7, 99.99, 4);

-- Insert random data (100 rows each)
-- Using deterministic seed for reproducibility
SELECT setseed(0.5);

INSERT INTO qgen_users (uuid, name, color, age, quantity, price, rating)
SELECT 
    rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid,
    (ARRAY ['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy']::text[])[(floor(random() * 7) + 1)::int],
    (ARRAY ['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow', NULL]::text[])[(floor(random() * 8) + 1)::int],
    (floor(random() * 100) + 1)::int,
    CASE WHEN random() < 0.1 THEN NULL ELSE (floor(random() * 100) + 1)::int END,
    (random() * 1000 + 10)::numeric(10,2),
    (floor(random() * 5) + 1)::int
FROM generate_series(1, 100);

INSERT INTO qgen_products (uuid, name, color, age, quantity, price, rating)
SELECT 
    rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid,
    (ARRAY ['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy']::text[])[(floor(random() * 7) + 1)::int],
    (ARRAY ['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow', NULL]::text[])[(floor(random() * 8) + 1)::int],
    (floor(random() * 100) + 1)::int,
    CASE WHEN random() < 0.1 THEN NULL ELSE (floor(random() * 100) + 1)::int END,
    (random() * 1000 + 10)::numeric(10,2),
    (floor(random() * 5) + 1)::int
FROM generate_series(1, 100);

ANALYZE qgen_users;
ANALYZE qgen_products;

-- TEST 8: Simple query without NOT (baseline - should work)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE qgen_users.name @@@ 'bob' 
ORDER BY qgen_users.id 
LIMIT 5;

SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE qgen_users.name @@@ 'bob' 
ORDER BY qgen_users.id 
LIMIT 5;

-- TEST 9: Query with NOT operator (this is where the bug occurred)
-- Error was: "could not read blocks 65536..65536: read only 0 of 8192 bytes"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE NOT (qgen_users.name @@@ 'bob') 
ORDER BY qgen_users.id 
LIMIT 5;

SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE NOT (qgen_users.name @@@ 'bob') 
ORDER BY qgen_users.id 
LIMIT 5;

-- TEST 10: OR with predicates spanning both tables
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE (qgen_products.name @@@ 'alice') OR (qgen_users.name @@@ 'bob')
ORDER BY qgen_users.id 
LIMIT 5;

SELECT qgen_users.id, qgen_users.name 
FROM qgen_users 
JOIN qgen_products ON qgen_users.age = qgen_products.age 
WHERE (qgen_products.name @@@ 'alice') OR (qgen_users.name @@@ 'bob')
ORDER BY qgen_users.id 
LIMIT 5;

-- =============================================================================
-- TEST 11: Multi-table predicates with fast fields
-- =============================================================================
-- This test demonstrates JoinScan handling multi-table predicates (conditions
-- that reference columns from both tables) when ALL referenced columns are
-- fast fields in their respective BM25 indexes.
--
-- Multi-table predicates like `p.price >= s.min_order_value` are supported
-- when both `price` and `min_order_value` are indexed as fast fields.
-- If any column is NOT a fast field, JoinScan will not be proposed.

-- Add min_order_value to suppliers for cross-relation comparison
ALTER TABLE suppliers ADD COLUMN min_order_value DECIMAL(10,2) DEFAULT 0;
UPDATE suppliers SET min_order_value = 50.00 WHERE id = 151;  -- TechCorp
UPDATE suppliers SET min_order_value = 15.00 WHERE id = 152;  -- GlobalSupply
UPDATE suppliers SET min_order_value = 30.00 WHERE id = 153;  -- FastParts
UPDATE suppliers SET min_order_value = 100.00 WHERE id = 154; -- QualityFirst

-- Recreate indexes with price, min_order_value, and join key columns as fast fields
DROP INDEX IF EXISTS products_bm25_idx;
DROP INDEX IF EXISTS suppliers_bm25_idx;

CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, description, supplier_id, price)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}, "price": {"fast": true}}');

CREATE INDEX suppliers_bm25_idx ON suppliers USING bm25 (id, name, contact_info, country, min_order_value)
WITH (key_field = 'id', numeric_fields = '{"min_order_value": {"fast": true}}');

-- Test case: Search predicate AND multi-table predicate (both columns are fast fields)
-- Products where description matches 'wireless' AND price >= supplier's min_order_value
-- JoinScan SHOULD be proposed because price and min_order_value are fast fields
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, p.price, s.name as supplier, s.min_order_value
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND p.price >= s.min_order_value
ORDER BY p.id
LIMIT 10;

-- Test case: Search predicate OR multi-table predicate (unified expression tree)
-- Products where EITHER description matches 'cable' OR price >= supplier's min_order_value
-- JoinScan SHOULD be proposed because all columns in the multi-table predicate are fast fields
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, p.price, s.name as supplier, s.min_order_value
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'cable'
   OR p.price >= s.min_order_value
LIMIT 10;

-- Verify correct results
SELECT p.id, p.name, p.price, s.name as supplier, s.min_order_value
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'cable'
   OR p.price >= s.min_order_value
ORDER BY p.id
LIMIT 10;

-- Test case: Multi-table predicate with NON-indexed column
-- This should NOT use JoinScan because category_id is not in the BM25 index
-- (demonstrating the rejection of non-fast-field multi-table predicates)
-- Note: category_id was added via ALTER TABLE but is NOT in the recreated index
ALTER TABLE products ADD COLUMN category_id INTEGER;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name as supplier
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND p.category_id > s.id  -- category_id is NOT in the BM25 index
LIMIT 10;

-- =============================================================================
-- TEST 12: Functions in cross-table predicates (native DF + UDF fallback)
-- =============================================================================
-- Exercise PredicateTranslator's FuncExpr + arithmetic-OpExpr paths inside a
-- multi-table predicate. Both sides' columns are fast fields, so JoinScan
-- should absorb these and the EXPLAIN should show `abs(...)` natively while
-- the arithmetic OpExpr (`-`) gets wrapped as `pdb_eval_expr_opexpr_*` via
-- `try_wrap_as_udf`.

-- 12a: abs() wrapping a cross-table arithmetic OpExpr
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.supplier_id, s.id AS supplier_pk
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND abs(p.supplier_id - s.id) >= 0
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.supplier_id, s.id AS supplier_pk
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND abs(p.supplier_id - s.id) >= 0
ORDER BY p.id
LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT p.id, p.supplier_id, s.id AS supplier_pk
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND abs(p.supplier_id - s.id) >= 0
ORDER BY p.id
LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- 12b: native FuncExpr on one side of a comparison, UDF-wrapped arithmetic
-- on the other. LHS `abs(p.supplier_id)` is a native FuncExpr over a Var;
-- RHS `(s.id * 2)` is an arithmetic OpExpr that `translate_op_expr`
-- doesn't handle, so it gets wrapped as `pdb_eval_expr_opexpr_*`. The outer
-- `<=` comparison stays native. EXPLAIN should show BOTH paths side by side.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.supplier_id, s.id AS supplier_pk
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND abs(p.supplier_id) <= (s.id * 2)
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.supplier_id, s.id AS supplier_pk
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND abs(p.supplier_id) <= (s.id * 2)
ORDER BY p.id
LIMIT 10;

SET paradedb.enable_join_custom_scan = off;
SELECT p.id, p.supplier_id, s.id AS supplier_pk
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'wireless'
  AND abs(p.supplier_id) <= (s.id * 2)
ORDER BY p.id
LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;
DROP TABLE IF EXISTS qgen_products CASCADE;
DROP TABLE IF EXISTS qgen_users CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
