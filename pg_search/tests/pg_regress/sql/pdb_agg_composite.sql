-- pdb.agg() on fields indexed as members of a ROW(...) composite. A member that
-- is a column keeps the column's value, so the join path reads it back like a
-- bare tokenizer cast, and the SQL GROUP BY over the same tables is the reference
-- for each answer. A member that does not hold a column's value is not read over
-- a join.

\i common/common_setup.sql

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS pagc_products, pagc_orders CASCADE;
DROP TYPE IF EXISTS pagc_product_fields;
CREATE TYPE pagc_product_fields AS (
    category pdb.literal,
    brand pdb.literal,
    price NUMERIC(10, 2),
    name_upper pdb.literal,
    stock BIGINT
);
CREATE TABLE pagc_products (
    id INTEGER PRIMARY KEY,
    description TEXT,
    legacy TEXT,
    name TEXT,
    category TEXT,
    maker TEXT,
    price NUMERIC(10, 2),
    stock INTEGER
);
-- The dropped column leaves a gap in the attribute numbers of the columns after it.
ALTER TABLE pagc_products DROP COLUMN legacy;
CREATE TABLE pagc_orders (
    id INTEGER PRIMARY KEY,
    product_id INTEGER
);

INSERT INTO pagc_products (id, description, name, category, maker, price, stock)
SELECT i,
       'item ' || i,
       (ARRAY['alice', 'bob', 'cloe'])[1 + i % 3],
       (ARRAY['shoes', 'hats'])[1 + i % 2],
       CASE WHEN i % 4 = 0 THEN NULL ELSE (ARRAY['acme', 'zenith', 'orbit'])[1 + i % 3] END,
       (i * 1.25)::numeric(10, 2),
       i
FROM generate_series(1, 12) AS i;
INSERT INTO pagc_orders (id, product_id)
SELECT i, 1 + i % 12
FROM generate_series(1, 24) AS i;

CREATE INDEX pagc_products_idx ON pagc_products USING paradedb (
    id,
    description,
    (ROW(category, maker, price, upper(name), stock)::pagc_product_fields),
    (maker::pdb.literal('alias=maker_cast'))
);
CREATE INDEX pagc_orders_idx ON pagc_orders USING paradedb (id, product_id);

-- Test 1: terms on a member over a join
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.category, COUNT(*)
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*)
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
GROUP BY p.category
ORDER BY p.category;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "category", "order": {"_key": "asc"}}}')
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "category", "order": {"_key": "asc"}}}')
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

-- Test 2: a member with NULLs named differently from its column, with a metric
-- on a NUMERIC member and a count of a third one
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.maker, COUNT(*), SUM(p.price), COUNT(p.category)
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
GROUP BY p.maker
ORDER BY p.maker;

SELECT p.maker, COUNT(*), SUM(p.price), COUNT(p.category)
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
GROUP BY p.maker
ORDER BY p.maker;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}, "aggs": {"total": {"sum": {"field": "price"}}, "categories": {"value_count": {"field": "category"}}}}')
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}, "aggs": {"total": {"sum": {"field": "price"}}, "categories": {"value_count": {"field": "category"}}}}')
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

-- Test 3: on a single table, a member key with a NUMERIC metric
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, SUM(price)
FROM pagc_products
WHERE description ||| 'item'
GROUP BY category
ORDER BY category;

SELECT category, SUM(price)
FROM pagc_products
WHERE description ||| 'item'
GROUP BY category
ORDER BY category;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "category", "order": {"_key": "asc"}}, "aggs": {"total": {"sum": {"field": "price"}}}}')
FROM pagc_products
WHERE description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "category", "order": {"_key": "asc"}}, "aggs": {"total": {"sum": {"field": "price"}}}}')
FROM pagc_products
WHERE description ||| 'item';

-- Test 4: one column indexed as a member and through a tokenizer cast, with
-- both fields in one spec. Each brand bucket has one cast bucket with its key.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}, "aggs": {"casts": {"terms": {"field": "maker_cast"}}}}')
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}, "aggs": {"casts": {"terms": {"field": "maker_cast"}}}}')
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

-- Test 5: the window form, which the Join Scan runs
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}}') OVER ()
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
ORDER BY p.id
LIMIT 2;

SELECT p.id, pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}}') OVER ()
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
ORDER BY p.id
LIMIT 2;

-- Test 6: a member computed from an expression is not read over a join
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "name_upper"}}')
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "name_upper"}}')
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

-- Test 7: a BIGINT member over an INTEGER column holds a cast value, so it is
-- not read over a join either
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"sum": {"field": "stock"}}')
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

SELECT pdb.agg('{"sum": {"field": "stock"}}')
FROM pagc_products p JOIN pagc_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

DROP TABLE pagc_products, pagc_orders;
DROP TYPE pagc_product_fields;
RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;
