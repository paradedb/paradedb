-- =====================================================================
-- Aggregate-on-JOIN via DataFusion Backend
-- =====================================================================
-- Tests aggregate functions (COUNT, SUM, AVG, MIN, MAX) on JOIN queries
-- executed via the DataFusion custom scan backend.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test Data Setup
-- =====================================================================

CREATE TABLE agg_join_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT,
    rating INTEGER
);

CREATE TABLE agg_join_tags (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    tag_name TEXT
);

INSERT INTO agg_join_products (description, category, price, rating) VALUES
    ('Laptop with fast processor', 'Electronics', 999.99, 5),
    ('Gaming laptop with RGB', 'Electronics', 1299.99, 5),
    ('Running shoes for athletes', 'Sports', 89.99, 4),
    ('Winter jacket warm', 'Clothing', 129.99, 3),
    ('Toy laptop for kids', 'Toys', 499.99, 2);

INSERT INTO agg_join_tags (product_id, tag_name) VALUES
    (1, 'tech'), (1, 'computer'),
    (2, 'tech'), (2, 'gaming'),
    (3, 'fitness'), (3, 'running'),
    (4, 'outdoor'),
    (5, 'tech'), (5, 'kids');

CREATE INDEX agg_join_products_idx ON agg_join_products
USING bm25 (id, description, category, price, rating)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}, "rating": {"fast": true}}'
);

CREATE INDEX agg_join_tags_idx ON agg_join_tags
USING bm25 (id, product_id, tag_name)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}}',
    text_fields='{"tag_name": {"fast": true}}'
);

-- =====================================================================
-- SECTION 1: Scalar Aggregates on JOIN (no GROUP BY)
-- =====================================================================

-- Test 1.1: COUNT(*) — verifies basic join + aggregate works
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 1.2: Multiple aggregates (COUNT, SUM, AVG)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 1.3: MIN/MAX
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT MIN(p.price), MAX(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT MIN(p.price), MAX(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 1.4: All five aggregate functions together
SELECT COUNT(*), SUM(p.price), AVG(p.price), MIN(p.rating), MAX(p.rating)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- =====================================================================
-- SECTION 2: Empty Result Sets
-- =====================================================================

-- Test 2.1: COUNT(*) on empty result — should return 0
SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent_term_xyz';

-- Test 2.2: SUM/AVG on empty result — should return NULL
SELECT SUM(p.price), AVG(p.rating)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent_term_xyz';

-- Test 2.3: MIN/MAX on empty result — should return NULL
SELECT MIN(p.price), MAX(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent_term_xyz';

-- =====================================================================
-- SECTION 3: Broader search predicates
-- =====================================================================

-- Test 3.1: Search matching all products (broader @@@ match)
SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy';

-- Test 3.2: COUNT of a specific column (not COUNT(*))
SELECT COUNT(p.rating)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 3.3: Filter on non-search column (@@@ combined with scalar predicate)
SELECT COUNT(*), SUM(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop' AND p.price > 500;

-- =====================================================================
-- SECTION 4: GROUP BY on JOIN (requires custom_scan_tlist for scanrelid=0)
-- =====================================================================

-- Test 4.1: Single-column GROUP BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 4.2: GROUP BY with multiple aggregates
SELECT p.category, COUNT(*), SUM(p.price), AVG(p.rating), MIN(p.price), MAX(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 4.3: Multi-column GROUP BY
SELECT p.category, t.tag_name, COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category, t.tag_name
ORDER BY p.category, t.tag_name;

-- Test 4.4: GROUP BY parity — DataFusion vs Postgres
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), SUM(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT p.category, COUNT(*), SUM(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- =====================================================================
-- SECTION 5: NULL handling in aggregates
-- =====================================================================

-- Test 5.1: Aggregate on column with NULL values (products without tags)
-- Insert a product with no tags to test NULL join behavior
INSERT INTO agg_join_products (description, category, price, rating)
VALUES ('Orphan product no tags', 'Misc', NULL, NULL);

-- COUNT(*) should not include the orphan (INNER JOIN excludes it)
SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR orphan';

-- SUM/AVG on nullable columns — the orphan is excluded by INNER JOIN
SELECT SUM(p.price), AVG(p.rating)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR orphan';

-- Clean up the orphan
DELETE FROM agg_join_products WHERE description = 'Orphan product no tags';

-- =====================================================================
-- SECTION 6: COUNT(DISTINCT) on JOIN
-- =====================================================================

-- Test 6.1: COUNT(DISTINCT) — routed through DataFusion via count_udaf(distinct=true).
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(DISTINCT t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(DISTINCT t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 6.2: COUNT(DISTINCT) parity — verify same result with custom scan off
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(DISTINCT t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- SECTION 7: LEFT JOIN aggregates
-- =====================================================================

-- Test 7.1: LEFT JOIN with COUNT — includes products with no tags
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(t.tag_name)
FROM agg_join_products p
LEFT JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

SELECT p.category, COUNT(t.tag_name)
FROM agg_join_products p
LEFT JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

-- Test 7.2: LEFT JOIN parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(t.tag_name), SUM(p.price)
FROM agg_join_products p
LEFT JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT p.category, COUNT(t.tag_name), SUM(p.price)
FROM agg_join_products p
LEFT JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- =====================================================================
-- SECTION 8: Composite ON clause (T_List equi-key extraction)
-- Postgres may wrap multi-condition ON clause quals in a T_List node
-- rather than a T_BoolExpr(AND). This tests that code path.
-- =====================================================================

CREATE TABLE comp_a (id SERIAL PRIMARY KEY, description TEXT, x INT, y INT);
CREATE TABLE comp_b (id SERIAL PRIMARY KEY, name TEXT, x INT, y INT);
INSERT INTO comp_a VALUES (1,'laptop fast',10,20),(2,'shoes nice',30,40),(3,'laptop pro',10,20);
INSERT INTO comp_b VALUES (1,'B1',10,20),(2,'B2',30,40);
CREATE INDEX idx_comp_a ON comp_a USING bm25(id,description,x,y) WITH (key_field='id',text_fields='{"description":{}}',numeric_fields='{"x":{"fast":true},"y":{"fast":true}}');
CREATE INDEX idx_comp_b ON comp_b USING bm25(id,name,x,y) WITH (key_field='id',text_fields='{"name":{}}',numeric_fields='{"x":{"fast":true},"y":{"fast":true}}');

-- Test 8.1: Composite ON with two equi-join keys — should use DataFusion
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*)
FROM comp_a a
JOIN comp_b b ON a.x = b.x AND a.y = b.y
WHERE a.description @@@ 'laptop OR shoes';

SELECT COUNT(*)
FROM comp_a a
JOIN comp_b b ON a.x = b.x AND a.y = b.y
WHERE a.description @@@ 'laptop OR shoes';

-- Test 8.2: Parity check for composite ON
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT COUNT(*)
FROM comp_a a
JOIN comp_b b ON a.x = b.x AND a.y = b.y
WHERE a.description @@@ 'laptop OR shoes';
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE comp_a;
DROP TABLE comp_b;

-- =====================================================================
-- SECTION 9: Verify single-table aggregates still use Tantivy
-- =====================================================================

-- Test 9.1: Single-table should show Tantivy backend (Index:, not Backend: DataFusion)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM agg_join_products WHERE description @@@ 'laptop';

SELECT COUNT(*) FROM agg_join_products WHERE description @@@ 'laptop';

-- =====================================================================
-- SECTION 10: Correctness parity — compare DataFusion vs Postgres default
-- =====================================================================

-- Test 10.1: Run the same query with custom scan OFF to verify result parity
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT COUNT(*), SUM(p.price), AVG(p.rating), MIN(p.price), MAX(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Restore
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- SECTION 10: Date/Timestamp aggregates on JOIN
-- =====================================================================

CREATE TABLE ts_orders (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    created_at TIMESTAMP
);

CREATE TABLE ts_items (
    id SERIAL PRIMARY KEY,
    order_id INTEGER,
    item_name TEXT
);

INSERT INTO ts_orders (description, category, created_at) VALUES
    ('Laptop order', 'Electronics', '2024-01-15 10:30:00'),
    ('Phone order', 'Electronics', '2024-03-20 14:45:00'),
    ('Shoes order', 'Sports', '2024-06-10 08:15:00');

INSERT INTO ts_items (order_id, item_name) VALUES
    (1, 'laptop'), (1, 'charger'),
    (2, 'phone'),
    (3, 'shoes'), (3, 'socks');

CREATE INDEX ts_orders_idx ON ts_orders
USING bm25 (id, description, category, created_at)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    datetime_fields='{"created_at": {"fast": true}}'
);

CREATE INDEX ts_items_idx ON ts_items
USING bm25 (id, order_id, item_name)
WITH (
    key_field='id',
    numeric_fields='{"order_id": {"fast": true}}',
    text_fields='{"item_name": {"fast": true}}'
);

-- Test 10.1: MIN/MAX on timestamp column via join
SELECT MIN(o.created_at), MAX(o.created_at)
FROM ts_orders o
JOIN ts_items i ON o.id = i.order_id
WHERE o.description @@@ 'order';

-- Test 10.2: GROUP BY with timestamp aggregate
SELECT o.category, MIN(o.created_at), MAX(o.created_at)
FROM ts_orders o
JOIN ts_items i ON o.id = i.order_id
WHERE o.description @@@ 'order'
GROUP BY o.category;

DROP TABLE ts_items;
DROP TABLE ts_orders;

-- =====================================================================
-- Clean up
-- =====================================================================
DROP TABLE agg_join_tags;
DROP TABLE agg_join_products;
