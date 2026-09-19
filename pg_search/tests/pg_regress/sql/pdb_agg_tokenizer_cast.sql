-- pdb.agg() on fields indexed through a tokenizer cast. A bare cast keeps the
-- column's value, so the join path reads it back like a plain column, and the
-- SQL GROUP BY over the same join is the reference for each answer. A computed
-- expression has no column behind it and stays out.

\i common/common_setup.sql

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS pac_products, pac_orders CASCADE;
CREATE TABLE pac_products (
    id INTEGER PRIMARY KEY,
    description TEXT,
    name TEXT,
    color VARCHAR,
    category TEXT,
    tags TEXT[],
    price NUMERIC(10, 2)
);
CREATE TABLE pac_orders (
    id INTEGER PRIMARY KEY,
    product_id INTEGER,
    status TEXT
);

INSERT INTO pac_products (id, description, name, color, category, tags, price)
SELECT i,
       'item ' || i,
       (ARRAY['alice', 'bob', 'cloe'])[1 + i % 3],
       CASE WHEN i % 4 = 0 THEN NULL ELSE (ARRAY['red', 'blue'])[1 + i % 2] END,
       (ARRAY['home garden', 'toys'])[1 + i % 2],
       CASE WHEN i % 5 = 0 THEN NULL ELSE ARRAY['sale', (ARRAY['new', 'old'])[1 + i % 2]] END,
       (i * 1.25)::numeric(10, 2)
FROM generate_series(1, 12) AS i;
INSERT INTO pac_orders (id, product_id, status)
SELECT i, 1 + i % 12, (ARRAY['open', 'shipped'])[1 + i % 2]
FROM generate_series(1, 24) AS i;

CREATE INDEX pac_products_idx ON pac_products USING paradedb (
    id,
    description,
    (name::pdb.literal),
    (color::pdb.literal),
    (category::pdb.unicode_words('columnar=true')),
    (tags::pdb.literal),
    price,
    (upper(name)::pdb.literal('alias=name_upper'))
);
CREATE INDEX pac_orders_idx ON pac_orders USING paradedb (
    id,
    product_id,
    (status::pdb.literal('alias=order_status'))
);

-- Test 1: terms on a literal cast over a join
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.name, COUNT(*)
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
GROUP BY p.name
ORDER BY p.name;

SELECT p.name, COUNT(*)
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
GROUP BY p.name
ORDER BY p.name;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "name"}}')
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "name"}}')
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

-- Test 2: a VARCHAR cast with NULLs, under it an aliased cast from the other
-- table, with a NUMERIC metric and a cardinality on a tokenized cast
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.color, o.status, COUNT(*), SUM(p.price), COUNT(DISTINCT p.category)
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
GROUP BY p.color, o.status
ORDER BY p.color, o.status;

SELECT p.color, o.status, COUNT(*), SUM(p.price), COUNT(DISTINCT p.category)
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
GROUP BY p.color, o.status
ORDER BY p.color, o.status;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "color", "order": {"_key": "asc"}}, "aggs": {"by_status": {"terms": {"field": "order_status", "order": {"_key": "asc"}}, "aggs": {"total": {"sum": {"field": "price"}}, "categories": {"cardinality": {"field": "category"}}}}}}')
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "color", "order": {"_key": "asc"}}, "aggs": {"by_status": {"terms": {"field": "order_status", "order": {"_key": "asc"}}, "aggs": {"total": {"sum": {"field": "price"}}, "categories": {"cardinality": {"field": "category"}}}}}}')
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

-- Test 3: terms on an array cast over a join
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT tag, COUNT(*)
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id, unnest(p.tags) AS tag
WHERE p.description ||| 'item'
GROUP BY tag
ORDER BY tag;

SELECT tag, COUNT(*)
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id, unnest(p.tags) AS tag
WHERE p.description ||| 'item'
GROUP BY tag
ORDER BY tag;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "tags", "order": {"_key": "asc"}}}')
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "tags", "order": {"_key": "asc"}}}')
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

-- Test 4: a SQL GROUP BY beside pdb.agg(), both on casts
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT o.status, pdb.agg('{"terms": {"field": "name", "order": {"_key": "asc"}}}')
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
GROUP BY o.status
ORDER BY o.status;

SELECT o.status, pdb.agg('{"terms": {"field": "name", "order": {"_key": "asc"}}}')
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item'
GROUP BY o.status
ORDER BY o.status;

-- Test 5: on a single table, a cast key with a NUMERIC metric
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT name, SUM(price)
FROM pac_products
WHERE description ||| 'item'
GROUP BY name
ORDER BY name;

SELECT name, SUM(price)
FROM pac_products
WHERE description ||| 'item'
GROUP BY name
ORDER BY name;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "name", "order": {"_key": "asc"}}, "aggs": {"total": {"sum": {"field": "price"}}}}')
FROM pac_products
WHERE description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "name", "order": {"_key": "asc"}}, "aggs": {"total": {"sum": {"field": "price"}}}}')
FROM pac_products
WHERE description ||| 'item';

-- Test 6: a computed expression reads back on a single table, but not over a join
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "name_upper", "order": {"_key": "asc"}}}')
FROM pac_products
WHERE description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "name_upper", "order": {"_key": "asc"}}}')
FROM pac_products
WHERE description ||| 'item';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "name_upper"}}')
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

SELECT pdb.agg('{"terms": {"field": "name_upper"}}')
FROM pac_products p JOIN pac_orders o ON p.id = o.product_id
WHERE p.description ||| 'item';

DROP TABLE pac_products, pac_orders;
RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;
