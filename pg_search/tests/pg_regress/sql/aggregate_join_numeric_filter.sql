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

SELECT explain_redact_oid($$
SELECT count(*) FILTER (WHERE o.price > 50) AS over_fifty,
       count(*) FILTER (WHERE o.price > 12.34) AS over_cheapest,
       count(*) FILTER (WHERE o.amount > 0) AS positive_amount,
       count(*) FILTER (WHERE o.id > 1) AS later_orders
FROM agg_numeric_products p
JOIN agg_numeric_orders o ON o.name = p.name
WHERE p.id @@@ pdb.all()
$$);

SELECT count(*) FILTER (WHERE o.price > 50) AS over_fifty,
       count(*) FILTER (WHERE o.price > 12.34) AS over_cheapest,
       count(*) FILTER (WHERE o.amount > 0) AS positive_amount,
       count(*) FILTER (WHERE o.id > 1) AS later_orders
FROM agg_numeric_products p
JOIN agg_numeric_orders o ON o.name = p.name
WHERE p.id @@@ pdb.all();

-- A HAVING clause that compares an aggregate with a NUMERIC literal: AVG of an
-- integer is NUMERIC, so 1.5 is a NUMERIC constant, decoded as its value.
-- bob's order ids average exactly 1.5 and carol's 3, so only carol is kept.
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

-- #6792 on a single table, with default settings so the DataFusion backend
-- takes the query: a NUMERIC FILTER declines, and the count is 1380, not the
-- 5400 rows the scan returns.
RESET paradedb.enable_custom_scan;
RESET paradedb.enable_filter_pushdown;
RESET enable_seqscan;
RESET enable_indexscan;

CREATE TABLE agg_numeric_scores (id text PRIMARY KEY, score numeric(3, 2), removed_at timestamptz);
INSERT INTO agg_numeric_scores
SELECT 'r' || g, (g % 100) / 10.0, CASE WHEN g % 10 = 0 THEN now() END
FROM generate_series(1, 6000) g;
CREATE INDEX agg_numeric_scores_idx ON agg_numeric_scores
USING paradedb ((id::pdb.literal), score) WHERE removed_at IS NULL;

EXPLAIN (COSTS OFF)
SELECT count(*) FILTER (WHERE score >= 7.5) AS high, count(score) AS n
FROM agg_numeric_scores
WHERE removed_at IS NULL;

SELECT count(*) FILTER (WHERE score >= 7.5) AS high, count(score) AS n
FROM agg_numeric_scores
WHERE removed_at IS NULL;

DROP TABLE agg_numeric_scores;

DROP TABLE agg_numeric_orders, agg_numeric_products;

RESET max_parallel_workers_per_gather;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_custom_scan;
RESET paradedb.enable_filter_pushdown;
RESET paradedb.enable_join_custom_scan;
RESET enable_seqscan;
RESET enable_indexscan;
