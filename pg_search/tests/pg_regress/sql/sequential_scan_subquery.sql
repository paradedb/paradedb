BEGIN;
CREATE SCHEMA sequential_scan_subquery;
SET search_path = sequential_scan_subquery, public;

CREATE TABLE products (id bigint PRIMARY KEY, name text, rating int, color text);
CREATE TABLE orders (id bigint PRIMARY KEY, name text, rating int, color text);
CREATE INDEX products_search ON products USING paradedb
    (id, (name::pdb.literal), (color::pdb.literal));
CREATE INDEX orders_search ON orders USING paradedb
    (id, (name::pdb.literal), (color::pdb.literal));

-- The same CTID names different rows in the two tables.
INSERT INTO products VALUES (1, 'bob', 4, 'blue'), (2, 'alice', 4, 'red');
INSERT INTO orders VALUES (1, 'alice', 4, 'red'), (2, 'bob', 4, 'blue');
ANALYZE products;
ANALYZE orders;
SET paradedb.enable_custom_scan = off;
SET paradedb.enable_custom_scan_without_operator = off;
SET paradedb.enable_aggregate_custom_scan = off;
SET paradedb.enable_join_custom_scan = off;
SET paradedb.enable_filter_pushdown = off;
SET enable_indexscan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;
SET enable_seqscan = off;
SET max_parallel_workers_per_gather = 0;

SELECT count(*) FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name AND NOT (orders.name = 'bob'))
      AND products.rating = 4 OR products.color = 'blue';
SELECT count(*) FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name AND NOT (orders.name === 'bob'))
      AND products.rating = 4 OR products.color === 'blue';

DO $$
DECLARE
    plan json;
BEGIN
    EXECUTE $query$EXPLAIN (FORMAT JSON, COSTS OFF)
        SELECT count(*) FROM products
        WHERE EXISTS (SELECT 1 FROM orders
                      WHERE orders.name = products.name AND NOT (orders.name === 'bob'))
              AND products.rating = 4 OR products.color === 'blue'$query$ INTO plan;
    ASSERT plan #>> '{0,Plan,Plans,0,Node Type}' = 'Seq Scan';
    ASSERT plan #>> '{0,Plan,Plans,0,Filter}' LIKE '%hashed SubPlan%',
        'the regression must exercise hashed EXISTS conversion';
END;
$$;

-- A generic plan keeps the RHS as an expression rather than a query constant.
SET plan_cache_mode = force_generic_plan;
PREPARE lookup(text) AS
SELECT count(*) FROM products
WHERE EXISTS (SELECT 1 FROM orders
              WHERE orders.name = products.name AND NOT (orders.name === $1))
      AND products.rating = 4 OR products.color === 'blue';
EXECUTE lookup('bob');
EXECUTE lookup('alice');
DEALLOCATE lookup;

-- Rebinding must not discard a user-supplied index expression with side effects.
CREATE SEQUENCE index_lookup_calls;
SELECT count(*) FROM products
WHERE id @@@ paradedb.with_index(
    CASE WHEN nextval('index_lookup_calls') > 0
         THEN 'products_search'::regclass ELSE 'orders_search'::regclass END,
    paradedb.term('color', 'blue')
);
SELECT is_called FROM index_lookup_calls;
ROLLBACK;
