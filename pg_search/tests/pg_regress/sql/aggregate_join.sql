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

-- Test 6.3: COUNT(DISTINCT) on the other join side (products table column)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT t.tag_name, COUNT(DISTINCT p.category)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY t.tag_name
ORDER BY t.tag_name;

SELECT t.tag_name, COUNT(DISTINCT p.category)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY t.tag_name
ORDER BY t.tag_name;

-- Test 6.4: COUNT(DISTINCT) on other side — parity
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT t.tag_name, COUNT(DISTINCT p.category)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY t.tag_name
ORDER BY t.tag_name;

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

-- Test 7.3: RIGHT JOIN with COUNT(left column) — includes tags without products
INSERT INTO agg_join_tags (product_id, tag_name) VALUES (999, 'orphan_tag');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT t.tag_name, COUNT(p.category)
FROM agg_join_products p
RIGHT JOIN agg_join_tags t ON p.id = t.product_id
WHERE t.tag_name @@@ 'tech OR orphan_tag'
GROUP BY t.tag_name
ORDER BY t.tag_name;

SELECT t.tag_name, COUNT(p.category)
FROM agg_join_products p
RIGHT JOIN agg_join_tags t ON p.id = t.product_id
WHERE t.tag_name @@@ 'tech OR orphan_tag'
GROUP BY t.tag_name
ORDER BY t.tag_name;

-- Test 7.4: RIGHT JOIN parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT t.tag_name, COUNT(p.category)
FROM agg_join_products p
RIGHT JOIN agg_join_tags t ON p.id = t.product_id
WHERE t.tag_name @@@ 'tech OR orphan_tag'
GROUP BY t.tag_name
ORDER BY t.tag_name;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT t.tag_name, COUNT(p.category)
FROM agg_join_products p
RIGHT JOIN agg_join_tags t ON p.id = t.product_id
WHERE t.tag_name @@@ 'tech OR orphan_tag'
GROUP BY t.tag_name
ORDER BY t.tag_name;

DELETE FROM agg_join_tags
WHERE product_id = 999 AND tag_name = 'orphan_tag';

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
-- SECTION 10: STDDEV/VARIANCE aggregates on JOIN
-- =====================================================================

-- Test 10.1: STDDEV and VARIANCE on join
SELECT STDDEV(p.price), VARIANCE(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- Test 10.2: STDDEV_POP and VAR_POP on join with GROUP BY
-- Uses p.price (not p.rating) so Electronics has actual variance (999.99 vs 1299.99)
SELECT p.category, STDDEV_POP(p.price), VAR_POP(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

-- Test 10.3: STDDEV parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT STDDEV(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT STDDEV(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- =====================================================================
-- SECTION 11: Date/Timestamp aggregates on JOIN
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

-- Parity check for TIMESTAMP (no TZ)
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT o.category, MIN(o.created_at), MAX(o.created_at)
FROM ts_orders o
JOIN ts_items i ON o.id = i.order_id
WHERE o.description @@@ 'order'
GROUP BY o.category;
SET paradedb.enable_aggregate_custom_scan TO on;

SELECT o.category, MIN(o.created_at), MAX(o.created_at)
FROM ts_orders o
JOIN ts_items i ON o.id = i.order_id
WHERE o.description @@@ 'order'
GROUP BY o.category;

DROP TABLE ts_items;
DROP TABLE ts_orders;

-- Test 10.3: TIMESTAMPTZ column via join
CREATE TABLE tstz_orders (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    created_at TIMESTAMPTZ
);

CREATE TABLE tstz_items (
    id SERIAL PRIMARY KEY,
    order_id INTEGER,
    item_name TEXT
);

INSERT INTO tstz_orders (description, category, created_at) VALUES
    ('Laptop order', 'Electronics', '2024-01-15 10:30:00+05:30'),
    ('Phone order', 'Electronics', '2024-03-20 14:45:00-04:00'),
    ('Shoes order', 'Sports', '2024-06-10 08:15:00+00:00'),
    ('Tablet order', 'Electronics', '2024-07-04 12:00:00 America/New_York'),
    ('Jacket order', 'Sports', '2024-12-25 00:00:00 Asia/Tokyo');

INSERT INTO tstz_items (order_id, item_name) VALUES
    (1, 'laptop'), (1, 'charger'),
    (2, 'phone'),
    (3, 'shoes'), (3, 'socks'),
    (4, 'tablet'),
    (5, 'jacket');

CREATE INDEX tstz_orders_idx ON tstz_orders
USING bm25 (id, description, category, created_at)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    datetime_fields='{"created_at": {"fast": true}}'
);

CREATE INDEX tstz_items_idx ON tstz_items
USING bm25 (id, order_id, item_name)
WITH (
    key_field='id',
    numeric_fields='{"order_id": {"fast": true}}',
    text_fields='{"item_name": {"fast": true}}'
);

-- MIN/MAX on TIMESTAMPTZ
SELECT MIN(o.created_at), MAX(o.created_at)
FROM tstz_orders o
JOIN tstz_items i ON o.id = i.order_id
WHERE o.description @@@ 'order';

-- GROUP BY with TIMESTAMPTZ aggregate
SELECT o.category, MIN(o.created_at), MAX(o.created_at)
FROM tstz_orders o
JOIN tstz_items i ON o.id = i.order_id
WHERE o.description @@@ 'order'
GROUP BY o.category;

-- Parity check: TIMESTAMPTZ results must match native PG
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT MIN(o.created_at), MAX(o.created_at)
FROM tstz_orders o
JOIN tstz_items i ON o.id = i.order_id
WHERE o.description @@@ 'order';
SET paradedb.enable_aggregate_custom_scan TO on;

SELECT MIN(o.created_at), MAX(o.created_at)
FROM tstz_orders o
JOIN tstz_items i ON o.id = i.order_id
WHERE o.description @@@ 'order';

-- Parity check: TIMESTAMPTZ GROUP BY results must match native PG.
-- The source data uses mixed timezones (+05:30, -04:00, UTC, America/New_York,
-- Asia/Tokyo) so any incorrect tz_opt propagation would show up as a mismatch.
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT o.category, MIN(o.created_at), MAX(o.created_at)
FROM tstz_orders o
JOIN tstz_items i ON o.id = i.order_id
WHERE o.description @@@ 'order'
GROUP BY o.category
ORDER BY o.category;
SET paradedb.enable_aggregate_custom_scan TO on;
SELECT o.category, MIN(o.created_at), MAX(o.created_at)
FROM tstz_orders o
JOIN tstz_items i ON o.id = i.order_id
WHERE o.description @@@ 'order'
GROUP BY o.category
ORDER BY o.category;

DROP TABLE tstz_items;
DROP TABLE tstz_orders;

-- =====================================================================
-- SECTION 11: ORDER BY aggregate NULLS FIRST / NULLS LAST
-- Tests that non-default NULL ordering in TopK produces correct results.
-- LEFT JOIN creates groups with NULL SUM (no right-side matches).
-- =====================================================================

-- Setup: product with no matching tags -> SUM(t.product_id) will be NULL for this group
INSERT INTO agg_join_products (id, description, price, category, rating)
VALUES (9901, 'nullsort test alpha', 10.00, 'NullSortA', 1.0);
INSERT INTO agg_join_products (id, description, price, category, rating)
VALUES (9902, 'nullsort test beta', 20.00, 'NullSortB', 2.0);
-- Give NullSortB some tags so its SUM is non-NULL
INSERT INTO agg_join_tags (product_id, tag_name) VALUES (9902, 'real_tag_1');
INSERT INTO agg_join_tags (product_id, tag_name) VALUES (9902, 'real_tag_2');

-- Rebuild the BM25 index so the new rows are visible
DROP INDEX agg_join_products_idx;
CREATE INDEX agg_join_products_idx ON agg_join_products
USING bm25 (id, description, category, price, rating)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}, "rating": {"fast": true}}'
);

-- Test 11.1: DESC NULLS LAST — non-NULL rows should come first
SELECT p.category, SUM(t.product_id)
FROM agg_join_products p
LEFT JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nullsort'
GROUP BY p.category
ORDER BY SUM(t.product_id) DESC NULLS LAST
LIMIT 2;

-- Parity check
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, SUM(t.product_id)
FROM agg_join_products p
LEFT JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nullsort'
GROUP BY p.category
ORDER BY SUM(t.product_id) DESC NULLS LAST
LIMIT 2;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 11.2: ASC NULLS FIRST — NULL rows should come first
SELECT p.category, SUM(t.product_id)
FROM agg_join_products p
LEFT JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nullsort'
GROUP BY p.category
ORDER BY SUM(t.product_id) ASC NULLS FIRST
LIMIT 2;

-- Parity check
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, SUM(t.product_id)
FROM agg_join_products p
LEFT JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nullsort'
GROUP BY p.category
ORDER BY SUM(t.product_id) ASC NULLS FIRST
LIMIT 2;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 11.3: Default orderings (should still work — regression guard)
-- DESC NULLS FIRST (default for DESC)
SELECT p.category, SUM(t.product_id)
FROM agg_join_products p
LEFT JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nullsort'
GROUP BY p.category
ORDER BY SUM(t.product_id) DESC
LIMIT 2;

-- Parity check
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, SUM(t.product_id)
FROM agg_join_products p
LEFT JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nullsort'
GROUP BY p.category
ORDER BY SUM(t.product_id) DESC
LIMIT 2;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Cleanup
DELETE FROM agg_join_tags WHERE product_id IN (9901, 9902);
DELETE FROM agg_join_products WHERE id IN (9901, 9902);

-- =====================================================================
-- SECTION 12: Non-COUNT DISTINCT aggregates fall back to native PG
-- Verifies SUM(DISTINCT), AVG(DISTINCT) route through DataFusion.
-- =====================================================================

-- SUM(DISTINCT) should show AggregateScan (DataFusion supports DISTINCT)
EXPLAIN (COSTS OFF)
SELECT p.category, SUM(DISTINCT t.product_id)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

-- AVG(DISTINCT) should show AggregateScan (DataFusion supports DISTINCT)
EXPLAIN (COSTS OFF)
SELECT p.category, AVG(DISTINCT t.product_id)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

-- COUNT(DISTINCT) should still show AggregateScan (this still works)
EXPLAIN (COSTS OFF)
SELECT p.category, COUNT(DISTINCT t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

-- =====================================================================
-- SECTION 13: Cross-table OR predicates (regression for generated_joins_small)
-- =====================================================================
-- Reproduces a proptest failure where COUNT(*) with an OR predicate spanning
-- two join tables returned a different result under the aggregate scan path.
-- The aggregate scan must not mishandle cross-table OR conditions.

-- Test 13.1: Cross-table OR — aggregate scan vs native Postgres
SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE (t.id = 1 OR p.id = 1) AND p.description @@@ 'laptop';

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE (t.id = 1 OR p.id = 1) AND p.description @@@ 'laptop';
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 13.2: Cross-table OR with @@@ on both sides
SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE (t.id @@@ '1' OR p.id @@@ '1') AND p.description @@@ 'laptop';

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE (t.id @@@ '1' OR p.id @@@ '1') AND p.description @@@ 'laptop';
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- SECTION 12: Post-join filter execution
-- =====================================================================

-- Test 12.1: Post-join filter with simple comparison
-- The price > 500 filter cannot be pushed to the scan level for the join,
-- so it should be applied as a DataFusion filter between join and aggregate.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), SUM(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop' AND p.price > 500;

SELECT COUNT(*), SUM(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop' AND p.price > 500;

-- Test 12.2: Post-join filter parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT COUNT(*), SUM(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop' AND p.price > 500;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- SECTION 13: HAVING clause support
-- =====================================================================

-- Test 13.1: HAVING COUNT(*) > N — DataFusion applies filter post-aggregate
SELECT p.category, COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
HAVING COUNT(*) > 1
ORDER BY p.category;

-- Test 13.2: HAVING parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
HAVING COUNT(*) > 1
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 13.3: HAVING with SUM — non-COUNT aggregate in HAVING
SELECT p.category, COUNT(*), SUM(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
HAVING SUM(p.price) > 100
ORDER BY p.category;

-- Test 13.4: HAVING SUM parity
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), SUM(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
HAVING SUM(p.price) > 100
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- SECTION 14: Additional aggregate functions (BOOL_AND/OR, ARRAY_AGG, STRING_AGG)
-- =====================================================================

-- Add a boolean column for BOOL_AND/OR tests
ALTER TABLE agg_join_products ADD COLUMN in_stock BOOLEAN DEFAULT true;
UPDATE agg_join_products SET in_stock = false WHERE category = 'Toys';

-- We need fast field access for in_stock; recreate BM25 index
DROP INDEX agg_join_products_idx;
CREATE INDEX agg_join_products_idx ON agg_join_products
USING bm25 (id, description, category, price, rating, in_stock)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}, "rating": {"fast": true}}',
    boolean_fields='{"in_stock": {"fast": true}}'
);

-- Test 14.1: BOOL_AND on join
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, BOOL_AND(p.in_stock)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR toy'
GROUP BY p.category;

SELECT p.category, BOOL_AND(p.in_stock)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR toy'
GROUP BY p.category
ORDER BY p.category;

-- Test 14.2: BOOL_OR on join
SELECT p.category, BOOL_OR(p.in_stock)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR toy'
GROUP BY p.category
ORDER BY p.category;

-- Test 14.3: STRING_AGG on join
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, STRING_AGG(t.tag_name, ', ')
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

SELECT p.category, STRING_AGG(t.tag_name, ', ')
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 14.4: BOOL_AND/OR parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, BOOL_AND(p.in_stock), BOOL_OR(p.in_stock)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR toy'
GROUP BY p.category
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT p.category, BOOL_AND(p.in_stock), BOOL_OR(p.in_stock)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR toy'
GROUP BY p.category
ORDER BY p.category;

-- Test 13.5: ARRAY_AGG on join
SELECT p.category, ARRAY_AGG(t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 13.6: ARRAY_AGG parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, ARRAY_AGG(t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO on;

-- Clean up the added column (drop+recreate index)
DROP INDEX agg_join_products_idx;
ALTER TABLE agg_join_products DROP COLUMN in_stock;
CREATE INDEX agg_join_products_idx ON agg_join_products
USING bm25 (id, description, category, price, rating)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}, "rating": {"fast": true}}'
);

-- =====================================================================
-- SECTION 15: FULL OUTER JOIN aggregates
-- =====================================================================

-- Test 15.1: FULL OUTER JOIN with COUNT — includes unmatched rows from both sides
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), COUNT(p.category), COUNT(t.tag_name)
FROM agg_join_products p
FULL OUTER JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

SELECT COUNT(*), COUNT(p.category), COUNT(t.tag_name)
FROM agg_join_products p
FULL OUTER JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- Test 15.2: FULL OUTER JOIN with GROUP BY
SELECT p.category, COUNT(*), SUM(p.price)
FROM agg_join_products p
FULL OUTER JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 15.3: FULL OUTER JOIN parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), SUM(p.price)
FROM agg_join_products p
FULL OUTER JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT p.category, COUNT(*), SUM(p.price)
FROM agg_join_products p
FULL OUTER JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- =====================================================================
-- SECTION 16: ORDER BY within aggregates (STRING_AGG, ARRAY_AGG)
-- =====================================================================

-- Test 16.1: STRING_AGG with ORDER BY on join
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, STRING_AGG(t.tag_name, ', ' ORDER BY t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

SELECT p.category, STRING_AGG(t.tag_name, ', ' ORDER BY t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 16.2: STRING_AGG ORDER BY parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, STRING_AGG(t.tag_name, ', ' ORDER BY t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 16.3: STRING_AGG ORDER BY DESC
SELECT p.category, STRING_AGG(t.tag_name, ', ' ORDER BY t.tag_name DESC)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 16.4: ARRAY_AGG with ORDER BY on join
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, ARRAY_AGG(t.tag_name ORDER BY t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

SELECT p.category, ARRAY_AGG(t.tag_name ORDER BY t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 16.5: ARRAY_AGG ORDER BY parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, ARRAY_AGG(t.tag_name ORDER BY t.tag_name)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 16.6: ARRAY_AGG ORDER BY DESC
SELECT p.category, ARRAY_AGG(t.tag_name ORDER BY t.tag_name DESC)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- =====================================================================
-- SECTION 17: JSON sub-field GROUP BY on join (DataFusion path)
-- =====================================================================

CREATE TABLE agg_json_items (
    id SERIAL PRIMARY KEY,
    metadata JSONB
);

CREATE TABLE agg_json_orders (
    id SERIAL PRIMARY KEY,
    item_id INTEGER,
    qty INTEGER
);

INSERT INTO agg_json_items (metadata) VALUES
    ('{"category": "Electronics", "brand": "Acme"}'),
    ('{"category": "Electronics", "brand": "Beta"}'),
    ('{"category": "Toys", "brand": "Acme"}');

INSERT INTO agg_json_orders (item_id, qty) VALUES
    (1, 10), (1, 5), (2, 3), (3, 7);

CREATE INDEX agg_json_items_idx ON agg_json_items
USING bm25 (id, metadata)
WITH (
    key_field='id',
    json_fields='{"metadata": {"fast": true}}'
);

CREATE INDEX agg_json_orders_idx ON agg_json_orders
USING bm25 (id, item_id, qty)
WITH (
    key_field='id',
    numeric_fields='{"item_id": {"fast": true}, "qty": {"fast": true}}'
);

-- Test 17.1: GROUP BY JSON sub-field on join — EXPLAIN shows DataFusion
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT i.metadata->>'category' AS category, COUNT(*), SUM(o.qty)
FROM agg_json_items i
JOIN agg_json_orders o ON i.id = o.item_id
WHERE i.id @@@ paradedb.all()
GROUP BY i.metadata->>'category';

-- Test 17.2: JSON sub-field GROUP BY results
SELECT i.metadata->>'category' AS category, COUNT(*), SUM(o.qty)
FROM agg_json_items i
JOIN agg_json_orders o ON i.id = o.item_id
WHERE i.id @@@ paradedb.all()
GROUP BY i.metadata->>'category'
ORDER BY category;

-- Test 17.3: JSON sub-field GROUP BY parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT i.metadata->>'category' AS category, COUNT(*), SUM(o.qty)
FROM agg_json_items i
JOIN agg_json_orders o ON i.id = o.item_id
WHERE i.id @@@ paradedb.all()
GROUP BY i.metadata->>'category'
ORDER BY category;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE agg_json_orders;
DROP TABLE agg_json_items;

-- =====================================================================
-- Clean up
-- =====================================================================
DROP TABLE agg_join_tags;
DROP TABLE agg_join_products;
