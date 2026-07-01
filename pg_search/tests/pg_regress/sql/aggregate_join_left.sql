-- =====================================================================
-- Aggregate-on-join: LEFT JOIN support
--
-- Verifies that LEFT JOIN queries use the ParadeDB Aggregate Scan path
-- and produce correct results, including NULL-extended rows for
-- unmatched left-side tuples.
--
-- Coverage in this file:
--   Test 1: Basic LEFT JOIN COUNT(*) - pushes down to Aggregate Scan
--           and returns correct count including unmatched left rows.
--   Test 2: Parity check - LEFT JOIN with aggregate scan OFF must match.
--   Test 3: LEFT JOIN where no right rows match - all left rows are
--           preserved with NULL right side. COUNT must equal left count.
--   Test 4: LEFT JOIN with multiple aggregates (COUNT, SUM, AVG).
--   Test 5: LEFT JOIN with GROUP BY on the left (preserved) side.
--   Test 6: The star-schema shape from the original issue report -
--           large fact table LEFT JOIN small dim table, WHERE predicate
--           on the fact (left/preserved) side.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

-- =====================================================================
-- Test data
-- =====================================================================

CREATE TABLE ajl_fact (
    id      bigint PRIMARY KEY,
    dim_id  int,
    notes   text,
    amount  numeric
);

CREATE TABLE ajl_dim (
    id   int PRIMARY KEY,
    name text
);

-- fact rows 1-6: dim_id 1 or 2 (will match dim)
-- fact rows 7-9: dim_id 99 (no matching dim row -> NULL-extended)
INSERT INTO ajl_fact VALUES
    (1, 1, 'lorem ipsum alpha',   10),
    (2, 1, 'lorem ipsum beta',    20),
    (3, 2, 'lorem ipsum gamma',   30),
    (4, 2, 'lorem delta',         40),
    (5, 1, 'lorem ipsum epsilon', 50),
    (6, 2, 'lorem ipsum zeta',    60),
    (7, 99, 'lorem ipsum eta',    70),
    (8, 99, 'lorem ipsum theta',  80),
    (9, 99, 'other text',          5);

INSERT INTO ajl_dim VALUES
    (1, 'widget'),
    (2, 'gadget');

CREATE INDEX ajl_fact_idx ON ajl_fact
USING bm25 (id, dim_id, notes, amount)
WITH (
    key_field = 'id',
    numeric_fields = '{"dim_id": {"fast": true}, "amount": {"fast": true}}',
    text_fields   = '{"notes": {}}'
);

CREATE INDEX ajl_dim_idx ON ajl_dim
USING bm25 (id, name)
WITH (key_field = 'id', text_fields = '{"name": {}}');

ANALYZE ajl_fact;
ANALYZE ajl_dim;

-- =====================================================================
-- Test 1: Basic LEFT JOIN COUNT(*) with a WHERE on the preserved side.
--
-- fact rows matching `notes @@@ 'lorem'`: ids 1-8 (9 does not match).
-- Of those 8 rows: ids 7 and 8 have dim_id=99 (no dim match) -> NULL.
-- A LEFT JOIN must preserve all 8 matching rows, so COUNT = 8.
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*)
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id
WHERE f.notes @@@ 'lorem';

SELECT COUNT(*)
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id
WHERE f.notes @@@ 'lorem';

-- =====================================================================
-- Test 2: Parity - same query with aggregate custom scan OFF must
--         return the same count as Test 1.
-- =====================================================================
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT COUNT(*)
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id
WHERE f.notes @@@ 'lorem';

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 3: LEFT JOIN where NO right rows can match (empty dim subset).
--         All left rows matching the WHERE are preserved; COUNT equals
--         the number of matching fact rows regardless of the join.
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*)
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id AND d.id = -1
WHERE f.notes @@@ 'lorem';

SELECT COUNT(*)
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id AND d.id = -1
WHERE f.notes @@@ 'lorem';

SET paradedb.enable_aggregate_custom_scan TO off;

SELECT COUNT(*)
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id AND d.id = -1
WHERE f.notes @@@ 'lorem';

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 4: LEFT JOIN with multiple aggregates (COUNT, SUM, AVG).
--         Rows matching `notes @@@ 'ipsum'`: ids 1,2,3,5,6,7,8.
--         SUM(amount) = 10+20+30+50+60+70+80 = 320.
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*), SUM(f.amount), AVG(f.amount)
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id
WHERE f.notes @@@ 'ipsum';

SELECT COUNT(*), SUM(f.amount), AVG(f.amount)
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id
WHERE f.notes @@@ 'ipsum';

SET paradedb.enable_aggregate_custom_scan TO off;

SELECT COUNT(*), SUM(f.amount), AVG(f.amount)
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id
WHERE f.notes @@@ 'ipsum';

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 5: LEFT JOIN with GROUP BY on the preserved (left) side.
--         Groups by dim_id; unmatched rows (dim_id=99) form their own
--         group since the left column is still accessible.
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT f.dim_id, COUNT(*) AS cnt
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id
WHERE f.notes @@@ 'lorem'
GROUP BY f.dim_id
ORDER BY f.dim_id;

SELECT f.dim_id, COUNT(*) AS cnt
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id
WHERE f.notes @@@ 'lorem'
GROUP BY f.dim_id
ORDER BY f.dim_id;

SET paradedb.enable_aggregate_custom_scan TO off;

SELECT f.dim_id, COUNT(*) AS cnt
FROM ajl_fact f
LEFT JOIN ajl_dim d ON f.dim_id = d.id
WHERE f.notes @@@ 'lorem'
GROUP BY f.dim_id
ORDER BY f.dim_id;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 6: Star-schema shape from the original issue report.
--         Large fact table LEFT JOIN small dim table, WHERE on the fact
--         (preserved) side. This is the primary motivating shape for
--         the feature. Asserts Aggregate Scan is chosen and that the
--         result matches the Postgres-only path.
-- =====================================================================
CREATE TABLE ajl_orders (
    id         bigint PRIMARY KEY,
    product_id int,
    region     text,
    revenue    numeric
);

CREATE TABLE ajl_products (
    id       int PRIMARY KEY,
    category text
);

INSERT INTO ajl_orders
SELECT s,
       CASE WHEN s % 3 = 0 THEN 999 ELSE (s % 5) + 1 END,
       CASE WHEN s % 2 = 0 THEN 'north' ELSE 'south' END,
       (s % 100) + 1
FROM generate_series(1, 200) s;

INSERT INTO ajl_products VALUES
    (1, 'electronics'),
    (2, 'clothing'),
    (3, 'food'),
    (4, 'toys'),
    (5, 'books');

CREATE INDEX ajl_orders_idx ON ajl_orders
USING bm25 (id, product_id, region, revenue)
WITH (
    key_field = 'id',
    numeric_fields = '{"product_id": {"fast": true}, "revenue": {"fast": true}}',
    text_fields   = '{"region": {"fast": true}}'
);

CREATE INDEX ajl_products_idx ON ajl_products
USING bm25 (id, category)
WITH (key_field = 'id', text_fields = '{"category": {}}');

ANALYZE ajl_orders;
ANALYZE ajl_products;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*), SUM(o.revenue)
FROM ajl_orders o
LEFT JOIN ajl_products p ON o.product_id = p.id
WHERE o.region @@@ 'north';

SELECT COUNT(*), SUM(o.revenue)
FROM ajl_orders o
LEFT JOIN ajl_products p ON o.product_id = p.id
WHERE o.region @@@ 'north';

SET paradedb.enable_aggregate_custom_scan TO off;

SELECT COUNT(*), SUM(o.revenue)
FROM ajl_orders o
LEFT JOIN ajl_products p ON o.product_id = p.id
WHERE o.region @@@ 'north';

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Cleanup
-- =====================================================================
DROP TABLE ajl_orders;
DROP TABLE ajl_products;
DROP TABLE ajl_fact;
DROP TABLE ajl_dim;
