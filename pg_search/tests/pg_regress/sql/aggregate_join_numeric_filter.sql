CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan = on;
SET paradedb.enable_custom_scan = off;
SET paradedb.enable_filter_pushdown = off;
SET paradedb.enable_join_custom_scan = off;
SET enable_seqscan = off;
SET enable_indexscan = off;

CREATE TABLE agg_numeric_products (id integer PRIMARY KEY, name text);
CREATE TABLE agg_numeric_orders (
    id integer PRIMARY KEY,
    name text,
    price numeric(10, 2),
    amount numeric
);

INSERT INTO agg_numeric_products VALUES (1, 'alice'), (2, 'bob'), (3, 'carol');
INSERT INTO agg_numeric_orders VALUES
    (1, 'bob', 99.99, 12345678901234567890.123456789),
    (2, 'bob', 12.34, -12345678901234567890.123456789),
    (3, 'carol', NULL, NULL);

CREATE INDEX agg_numeric_products_idx ON agg_numeric_products
USING paradedb (id, (name::pdb.literal));
CREATE INDEX agg_numeric_orders_idx ON agg_numeric_orders
USING paradedb (id, (name::pdb.literal), price, amount);

-- Residual numeric predicates must decode fast fields before PostgreSQL evaluation.
EXPLAIN (COSTS OFF)
SELECT count(*) FROM agg_numeric_products p
WHERE p.id @@@ pdb.all() AND EXISTS (
    SELECT 1 FROM agg_numeric_orders o
    WHERE (o.name = p.name AND o.name === 'bob')
       OR (o.name === 'bob' AND o.price = 99.99)
);

SELECT count(*) FROM agg_numeric_products p
WHERE p.id @@@ pdb.all() AND EXISTS (
    SELECT 1 FROM agg_numeric_orders o
    WHERE (o.name = p.name AND o.name === 'bob')
       OR (o.name === 'bob' AND o.price = 99.99)
);

SELECT count(*) FROM agg_numeric_products p
WHERE p.id @@@ pdb.all() AND EXISTS (
    SELECT 1 FROM agg_numeric_orders o
    WHERE (o.name = p.name AND o.name === 'bob')
       OR (o.name === 'bob' AND o.price = 9999)
);

EXPLAIN (COSTS OFF)
SELECT count(*) FROM agg_numeric_products p
WHERE p.id @@@ pdb.all() AND EXISTS (
    SELECT 1 FROM agg_numeric_orders o
    WHERE (o.name = p.name AND o.name === 'bob')
       OR (o.name === 'bob' AND o.amount = 12345678901234567890.123456789)
);

SELECT count(*) FROM agg_numeric_products p
WHERE p.id @@@ pdb.all() AND EXISTS (
    SELECT 1 FROM agg_numeric_orders o
    WHERE (o.name = p.name AND o.name === 'bob')
       OR (o.name === 'bob' AND o.amount = 12345678901234567890.123456789)
);

SELECT count(*) FROM agg_numeric_products p
WHERE p.id @@@ pdb.all() AND EXISTS (
    SELECT 1 FROM agg_numeric_orders o
    WHERE (o.name = p.name AND o.name === 'carol')
       OR (o.name === 'carol' AND o.amount = 12345678901234567890.123456789)
);

-- An aggregate FILTER that compares a NUMERIC column. The fast field holds the
-- storage encoding and the NUMERIC literal is not a float, so the scan must not
-- compare them directly. The join yields bob's orders (99.99, 12.34) and
-- carol's (NULL): Postgres counts 1, 1, 1 and 2.
EXPLAIN (COSTS OFF)
SELECT count(*) FILTER (WHERE o.price > 50) AS over_fifty,
       count(*) FILTER (WHERE o.price > 12.34) AS over_cheapest,
       count(*) FILTER (WHERE o.amount > 0) AS positive_amount,
       count(*) FILTER (WHERE o.id > 1) AS later_orders
FROM agg_numeric_products p
JOIN agg_numeric_orders o ON o.name = p.name
WHERE p.id @@@ pdb.all();

SELECT count(*) FILTER (WHERE o.price > 50) AS over_fifty,
       count(*) FILTER (WHERE o.price > 12.34) AS over_cheapest,
       count(*) FILTER (WHERE o.amount > 0) AS positive_amount,
       count(*) FILTER (WHERE o.id > 1) AS later_orders
FROM agg_numeric_products p
JOIN agg_numeric_orders o ON o.name = p.name
WHERE p.id @@@ pdb.all();

-- A HAVING clause that compares an aggregate with a NUMERIC literal: AVG of an
-- integer is NUMERIC, so 1.5 is a NUMERIC constant. bob's order ids average
-- exactly 1.5 and carol's 3, so only carol is kept.
EXPLAIN (COSTS OFF)
SELECT p.name, avg(o.id) AS avg_id
FROM agg_numeric_products p
JOIN agg_numeric_orders o ON o.name = p.name
WHERE p.id @@@ pdb.all()
GROUP BY p.name
HAVING avg(o.id) > 1.5
ORDER BY p.name;

SELECT p.name, avg(o.id) AS avg_id
FROM agg_numeric_products p
JOIN agg_numeric_orders o ON o.name = p.name
WHERE p.id @@@ pdb.all()
GROUP BY p.name
HAVING avg(o.id) > 1.5
ORDER BY p.name;

DROP TABLE agg_numeric_orders, agg_numeric_products;

RESET max_parallel_workers_per_gather;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_custom_scan;
RESET paradedb.enable_filter_pushdown;
RESET paradedb.enable_join_custom_scan;
RESET enable_seqscan;
RESET enable_indexscan;
