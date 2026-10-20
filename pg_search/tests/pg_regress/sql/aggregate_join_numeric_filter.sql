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

DROP TABLE agg_numeric_orders, agg_numeric_products;

RESET max_parallel_workers_per_gather;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_custom_scan;
RESET paradedb.enable_filter_pushdown;
RESET paradedb.enable_join_custom_scan;
RESET enable_seqscan;
RESET enable_indexscan;
