-- =====================================================================
-- Issue #7517: Aggregate FILTER with aliased index fields
-- =====================================================================
-- Verifies that FILTER Vars resolve through the heap-column-to-fast-field
-- mapping (not the heap attribute name) and register their fast fields by
-- stored attno, covering both:
--   1. An index alias that collides with another SQL column's name
--      (maker aliased as 'category', category aliased as 'brand').
--   2. An aliased column referenced only in FILTER (not in GROUP BY or
--      aggregate arguments).

CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_columnar_exec = on;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_aggregate_custom_scan = on;

CREATE TABLE filter_alias_products (
    id int PRIMARY KEY,
    category text,
    maker text
);

CREATE TABLE filter_alias_orders (
    id int PRIMARY KEY,
    product_id int
);

INSERT INTO filter_alias_products VALUES
    (1, 'hats', 'acme'),
    (2, 'shoes', 'zenith');

INSERT INTO filter_alias_orders VALUES
    (10, 1),
    (11, 1),
    (12, 2);

-- Indexed 'category' holds SQL column 'maker'; indexed 'brand' holds SQL
-- column 'category'.
CREATE INDEX filter_alias_products_idx ON filter_alias_products
USING paradedb (
    id,
    (maker::pdb.literal('alias=category')),
    (category::pdb.literal('alias=brand'))
);

CREATE INDEX filter_alias_orders_idx ON filter_alias_orders
USING paradedb (id, product_id);

-- =====================================================================
-- Test 1: pdb.agg() + FILTER when index alias collides with heap column
-- Previously read indexed 'category' (maker) instead of indexed 'brand'
-- (p.category) and returned {"value":0.0} instead of {"value":2.0}.
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"value_count":{"field":"category"}}')
       FILTER (WHERE p.category = 'hats') AS filtered_count
FROM filter_alias_products p
JOIN filter_alias_orders o ON p.id = o.product_id
WHERE p.id @@@ pdb.all();

SELECT pdb.agg('{"value_count":{"field":"category"}}')
       FILTER (WHERE p.category = 'hats') AS filtered_count
FROM filter_alias_products p
JOIN filter_alias_orders o ON p.id = o.product_id
WHERE p.id @@@ pdb.all();

-- Equivalent WHERE form
SELECT pdb.agg('{"value_count":{"field":"category"}}') AS where_count
FROM filter_alias_products p
JOIN filter_alias_orders o ON p.id = o.product_id
WHERE p.id @@@ pdb.all() AND p.category = 'hats';

-- Native Postgres reference
SET paradedb.enable_aggregate_custom_scan = off;
SELECT count(p.maker) FILTER (WHERE p.category = 'hats') AS native_count
FROM filter_alias_products p
JOIN filter_alias_orders o ON p.id = o.product_id;
SET paradedb.enable_aggregate_custom_scan = on;

-- =====================================================================
-- Test 2: Standard SQL aggregate with FILTER on aliased-only column
-- Ensures populate_required_fields registers FILTER columns by stored
-- attno even when the aliased column is not referenced elsewhere.
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT count(*) FILTER (WHERE p.category = 'hats') AS hat_orders,
       count(*) FILTER (WHERE p.maker = 'zenith') AS zenith_orders
FROM filter_alias_products p
JOIN filter_alias_orders o ON p.id = o.product_id
WHERE p.id @@@ pdb.all();

SELECT count(*) FILTER (WHERE p.category = 'hats') AS hat_orders,
       count(*) FILTER (WHERE p.maker = 'zenith') AS zenith_orders
FROM filter_alias_products p
JOIN filter_alias_orders o ON p.id = o.product_id
WHERE p.id @@@ pdb.all();

-- Native Postgres parity
SET paradedb.enable_aggregate_custom_scan = off;
SELECT count(*) FILTER (WHERE p.category = 'hats') AS hat_orders,
       count(*) FILTER (WHERE p.maker = 'zenith') AS zenith_orders
FROM filter_alias_products p
JOIN filter_alias_orders o ON p.id = o.product_id
WHERE p.id @@@ pdb.all();
SET paradedb.enable_aggregate_custom_scan = on;

-- =====================================================================
-- Test 3: GROUP BY + FILTER with aliased columns
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.maker, count(*) FILTER (WHERE p.category = 'hats') AS hat_orders
FROM filter_alias_products p
JOIN filter_alias_orders o ON p.id = o.product_id
WHERE p.id @@@ pdb.all()
GROUP BY p.maker
ORDER BY p.maker;

SELECT p.maker, count(*) FILTER (WHERE p.category = 'hats') AS hat_orders
FROM filter_alias_products p
JOIN filter_alias_orders o ON p.id = o.product_id
WHERE p.id @@@ pdb.all()
GROUP BY p.maker
ORDER BY p.maker;

-- Native Postgres parity
SET paradedb.enable_aggregate_custom_scan = off;
SELECT p.maker, count(*) FILTER (WHERE p.category = 'hats') AS hat_orders
FROM filter_alias_products p
JOIN filter_alias_orders o ON p.id = o.product_id
WHERE p.id @@@ pdb.all()
GROUP BY p.maker
ORDER BY p.maker;
SET paradedb.enable_aggregate_custom_scan = on;

-- Cleanup
DROP TABLE filter_alias_orders CASCADE;
DROP TABLE filter_alias_products CASCADE;
