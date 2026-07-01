-- Regression test for issue 5177.
-- When `WHERE` has a cross-table OR like `(u.name @@@ 'bob') OR (p.name @@@ 'bob')`
-- and PG pushes that OR into an inner sub-join's `joinrestrictinfo`, the outer
-- JoinScan's path reconstruction silently dropped `RestrictInfo` clauses that
-- contain `@@@`. The cross-table OR was then lost and the WHERE was effectively
-- dropped: BM25 returned join pairs where neither name matched 'bob'.

-- Disable parallel to keep the plan deterministic.
SET max_parallel_workers = 0;
SET max_parallel_workers_per_gather = 0;
SET parallel_leader_participation = off;

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS orders CASCADE;

CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
CREATE INDEX idxusers ON users USING bm25 (id, name)
  WITH (key_field = 'id',
        text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}');

CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, age INTEGER);
CREATE INDEX idxproducts ON products USING bm25 (id, name, age)
  WITH (key_field = 'id',
        text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}',
        numeric_fields = '{"age": {"fast": true}}');

CREATE TABLE orders (id INTEGER PRIMARY KEY, age INTEGER);
CREATE INDEX idxorders ON orders USING bm25 (id, age)
  WITH (key_field = 'id', numeric_fields = '{"age": {"fast": true}}');

-- Joinable on users.id = products.id and products.age = orders.age.
-- For each (u, p, o) triple, only some satisfy `u.name='bob' OR p.name='bob'`.
INSERT INTO users (id, name) VALUES
    (1, 'bob'),      -- satisfies via u.name
    (2, 'alice'),    -- not 'bob'
    (3, 'cloe'),     -- not 'bob'
    (4, 'brandy'),   -- not 'bob'
    (5, 'sally');    -- satisfies (because products.id=5 is 'bob')

INSERT INTO products (id, name, age) VALUES
    (1, 'cloe',   10),  -- name not 'bob', but pairs with u=1 ('bob') so OR holds
    (2, 'alice',  20),  -- neither 'bob'
    (3, 'sally',  30),  -- neither 'bob'
    (4, 'brisket',40),  -- neither 'bob'
    (5, 'bob',    50);  -- name='bob', satisfies OR

INSERT INTO orders (id, age) VALUES
    (101, 10),
    (102, 20),
    (103, 30),
    (104, 40),
    (105, 50);

ANALYZE users;
ANALYZE products;
ANALYZE orders;

SET paradedb.enable_aggregate_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_fast_field_exec = on;
SET paradedb.enable_mpp = off;

-- Force the (products INNER users) sub-join order so PG pushes the cross-table
-- OR down into the sub-join's `joinrestrictinfo`. Without this nesting the
-- planner places the OR on the outer join and the bug doesn't surface.
SET join_collapse_limit = 1;
SET from_collapse_limit = 1;

-- Reference plan via vanilla PG.
SET paradedb.enable_aggregate_custom_scan = off;
SET paradedb.enable_join_custom_scan = off;
SELECT 'PG' AS source, users.id AS uid, users.name AS uname,
       products.id AS pid, products.name AS pname, orders.id AS oid
FROM orders JOIN (products JOIN users ON users.id = products.id)
       ON products.age = orders.age
WHERE (users.name = 'bob') AND (NOT (products.name = 'bob'))
   OR (products.name = 'bob')
ORDER BY users.id, products.id, orders.id LIMIT 20;

-- BM25 path with JoinScan.
SET paradedb.enable_aggregate_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 'BM25' AS source, users.id AS uid, users.name AS uname,
       products.id AS pid, products.name AS pname, orders.id AS oid
FROM orders JOIN (products JOIN users ON users.id = products.id)
       ON products.age = orders.age
WHERE (users.name @@@ 'bob') AND (NOT (products.name @@@ 'bob'))
   OR (products.name @@@ 'bob')
ORDER BY users.id, products.id, orders.id LIMIT 20;

SELECT 'BM25' AS source, users.id AS uid, users.name AS uname,
       products.id AS pid, products.name AS pname, orders.id AS oid
FROM orders JOIN (products JOIN users ON users.id = products.id)
       ON products.age = orders.age
WHERE (users.name @@@ 'bob') AND (NOT (products.name @@@ 'bob'))
   OR (products.name @@@ 'bob')
ORDER BY users.id, products.id, orders.id LIMIT 20;

DROP TABLE users CASCADE;
DROP TABLE products CASCADE;
DROP TABLE orders CASCADE;
