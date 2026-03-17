\i common/common_setup.sql
CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;
SET paradedb.enable_columnar_exec = true;

-- Test TopN scan with indexed expressions (issue #3303)
-- Verifies that ORDER BY <expr> LIMIT N uses TopN scan when <expr> was
-- used to build the BM25 index, not just for the hardcoded patterns
-- (pdb.score(), lower(), plain col).

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

-- Test 1: FuncExpr - upper() indexed expression gets TopN scan
-- We search on category (separate column) and sort by upper(description).
CREATE INDEX upper_idx ON mock_items
    USING bm25 (id, category, (upper(description)::pdb.literal), rating)
    WITH (key_field='id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating FROM mock_items
WHERE category === 'electronics'
ORDER BY upper(description) DESC
    LIMIT 5;

SELECT description, rating FROM mock_items
WHERE category === 'electronics'
ORDER BY upper(description) DESC
    LIMIT 5;

DROP INDEX upper_idx;

-- Test 2: FuncExpr - trim() to verify it's not just upper() that works
CREATE INDEX trim_idx ON mock_items
    USING bm25 (id, category, (trim(description)::pdb.literal), rating)
    WITH (key_field='id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating FROM mock_items
WHERE category === 'electronics'
ORDER BY trim(description) DESC
    LIMIT 5;

SELECT description, rating FROM mock_items
WHERE category === 'electronics'
ORDER BY trim(description) DESC
    LIMIT 5;

DROP INDEX trim_idx;

-- Test 3: Negative test - expression NOT in index should NOT use TopN
CREATE INDEX plain_idx ON mock_items
    USING bm25 (id, description, rating)
    WITH (key_field='id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating FROM mock_items
WHERE description === 'sleek running shoes'
ORDER BY upper(description) DESC
    LIMIT 5;

DROP INDEX plain_idx;

-- Test 4: lower() still works as before (regression test)
CREATE INDEX lower_idx ON mock_items
    USING bm25 (id, (lower(description)::pdb.literal), rating)
    WITH (key_field='id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating FROM mock_items
WHERE description === 'sleek running shoes'
ORDER BY lower(description) DESC
    LIMIT 5;

SELECT description, rating FROM mock_items
WHERE description === 'sleek running shoes'
ORDER BY lower(description) DESC
    LIMIT 5;

DROP INDEX lower_idx;

-- Test 5: Mixed expression types - lower() + rating
CREATE INDEX mixed_idx ON mock_items
    USING bm25 (id, (lower(description)::pdb.literal), rating)
    WITH (key_field='id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating FROM mock_items
WHERE description === 'sleek running shoes'
ORDER BY lower(description) ASC, rating DESC
    LIMIT 5;

SELECT description, rating FROM mock_items
WHERE description === 'sleek running shoes'
ORDER BY lower(description) ASC, rating DESC
    LIMIT 5;

DROP INDEX mixed_idx;

-- Test 6: Mixed - indexed expression + plain column
CREATE INDEX mixed_expr_idx ON mock_items
    USING bm25 (id, category, (upper(description)::pdb.literal), rating)
    WITH (key_field='id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating FROM mock_items
WHERE category === 'electronics'
ORDER BY upper(description) ASC, rating DESC
    LIMIT 5;

SELECT description, rating FROM mock_items
WHERE category === 'electronics'
ORDER BY upper(description) ASC, rating DESC
    LIMIT 5;

DROP INDEX mixed_expr_idx;

DROP TABLE mock_items;

-- =============================================================================
-- Tests 7-9: JoinScan with indexed expressions
-- Exercises order_by_columns_are_fast_fields() and extract_orderby() changes.
-- =============================================================================

CREATE TABLE expr_products (
                               id INTEGER PRIMARY KEY,
                               name TEXT NOT NULL,
                               description TEXT,
                               category TEXT NOT NULL
);

CREATE TABLE expr_suppliers (
                                id INTEGER PRIMARY KEY,
                                product_id INTEGER NOT NULL,
                                name TEXT NOT NULL,
                                contact_info TEXT
);

INSERT INTO expr_products (id, name, description, category) VALUES
                                                                (1, 'Wireless Mouse', 'Ergonomic wireless mouse with Bluetooth', 'electronics'),
                                                                (2, 'USB Cable', 'High-speed USB-C cable for data transfer', 'accessories'),
                                                                (3, 'Keyboard', 'Mechanical keyboard with RGB wireless', 'electronics'),
                                                                (4, 'Monitor Stand', 'Adjustable monitor stand ergonomic', 'furniture'),
                                                                (5, 'Webcam', 'HD webcam for video conferencing', 'electronics'),
                                                                (6, 'Headphones', 'Wireless noise-canceling headphones', 'electronics'),
                                                                (7, 'Mouse Pad', 'Large gaming mouse pad wireless charging', 'accessories'),
                                                                (8, 'Cable Organizer', 'Desktop cable organizer for setup', 'furniture'),
                                                                (9, 'Wireless Charger', 'Fast wireless charging pad', 'electronics'),
                                                                (10, 'USB Hub', 'Multi-port USB hub for connectivity', 'accessories');

INSERT INTO expr_suppliers (id, product_id, name, contact_info) VALUES
                                                                    (1, 1, 'TechCorp', 'wireless technology'),
                                                                    (2, 2, 'CableCo', 'cable manufacturing'),
                                                                    (3, 3, 'TechCorp', 'wireless technology'),
                                                                    (4, 4, 'FurniPro', 'furniture solutions'),
                                                                    (5, 5, 'TechCorp', 'wireless technology'),
                                                                    (6, 6, 'TechCorp', 'wireless technology'),
                                                                    (7, 7, 'CableCo', 'cable manufacturing'),
                                                                    (8, 8, 'FurniPro', 'furniture solutions'),
                                                                    (9, 9, 'TechCorp', 'wireless technology'),
                                                                    (10, 10, 'CableCo', 'cable manufacturing');

-- BM25 indexes with fast fields and an indexed expression: upper(name)
CREATE INDEX expr_products_bm25 ON expr_products
    USING bm25 (id, description, category, (upper(name)::pdb.literal))
    WITH (
    key_field = 'id',
    text_fields = '{"category": {"fast": true}}'
    );

CREATE INDEX expr_suppliers_bm25 ON expr_suppliers
    USING bm25 (id, product_id, name, contact_info)
    WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true}}',
    numeric_fields = '{"product_id": {"fast": true}}'
    );

SET paradedb.enable_join_custom_scan = on;

-- Test 7: JoinScan with ORDER BY indexed expression
-- upper(name) is an indexed expression on expr_products — JoinScan should
-- push the sort via extract_orderby()'s new indexed-expression branch.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.name, s.name AS supplier_name
FROM expr_products p
         JOIN expr_suppliers s ON p.id = s.product_id
WHERE p.description @@@ 'wireless'
ORDER BY upper(p.name) ASC
    LIMIT 5;

SELECT p.name, s.name AS supplier_name
FROM expr_products p
         JOIN expr_suppliers s ON p.id = s.product_id
WHERE p.description @@@ 'wireless'
ORDER BY upper(p.name) ASC
    LIMIT 5;

-- Test 8: JoinScan with ORDER BY plain fast-field column (regression)
-- Ensures the existing Var path in order_by_columns_are_fast_fields()
-- is not broken by the new else branch.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.name, s.name AS supplier_name
FROM expr_products p
         JOIN expr_suppliers s ON p.id = s.product_id
WHERE p.description @@@ 'wireless'
ORDER BY s.name ASC
    LIMIT 5;

SELECT p.name, s.name AS supplier_name
FROM expr_products p
         JOIN expr_suppliers s ON p.id = s.product_id
WHERE p.description @@@ 'wireless'
ORDER BY s.name ASC
    LIMIT 5;

-- Test 9: JoinScan negative — upper(category) is NOT an indexed expression.
-- order_by_columns_are_fast_fields() should let this through (permissive else),
-- but extract_orderby() should fail to resolve it, so the sort won't be pushed.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.name, s.name AS supplier_name
FROM expr_products p
         JOIN expr_suppliers s ON p.id = s.product_id
WHERE p.description @@@ 'wireless'
ORDER BY upper(p.category) ASC
    LIMIT 5;

SELECT p.name, s.name AS supplier_name
FROM expr_products p
         JOIN expr_suppliers s ON p.id = s.product_id
WHERE p.description @@@ 'wireless'
ORDER BY upper(p.category) ASC
    LIMIT 5;

RESET paradedb.enable_join_custom_scan;

DROP INDEX expr_products_bm25;
DROP INDEX expr_suppliers_bm25;
DROP TABLE expr_suppliers;
DROP TABLE expr_products;
\i common/common_cleanup.sql