-- A `WHERE` clause that Postgres delays to an Anti join level is not a join
-- condition. Postgres evaluates it on the join's output rows, where the inner side
-- is null-extended. Folding it into the anti condition changes which pairs match
-- and lets rows through that Postgres drops.

\i common/common_setup.sql

DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS orders CASCADE;

CREATE TABLE users    (id SERIAL8 PRIMARY KEY, name TEXT, age INTEGER);
CREATE TABLE products (id SERIAL8 PRIMARY KEY, name TEXT, age INTEGER);
CREATE TABLE orders   (id SERIAL8 PRIMARY KEY, age INTEGER, note TEXT);

CREATE INDEX idxusers ON users USING bm25 (id, name, age)
  WITH (key_field='id', text_fields='{"name":{"fast":true}}', numeric_fields='{"age":{"fast":true}}');
CREATE INDEX idxproducts ON products USING bm25 (id, name, age)
  WITH (key_field='id', text_fields='{"name":{"fast":true}}', numeric_fields='{"age":{"fast":true}}');
CREATE INDEX idxorders ON orders USING bm25 (id, age, note)
  WITH (key_field='id', text_fields='{"note":{"fast":true}}', numeric_fields='{"age":{"fast":true}}');

-- `bob` matches product 2 on the anti condition alone, so Postgres drops it.
INSERT INTO users (name, age)    VALUES ('alice', NULL), ('bob', 5);
INSERT INTO products (name, age) VALUES ('alice', 3), ('bob', 4);
INSERT INTO orders (age, note)   VALUES (7, 'urgent'), (8, 'later');

SET paradedb.enable_join_custom_scan TO on;

-- The reported query. `COALESCE(users.age, 0) = 0` is TRUE on every output row.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND orders.note @@@ 'urgent OR later'
  AND COALESCE(users.age, 0) = 0
ORDER BY 2, 3
LIMIT 10;

SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND orders.note @@@ 'urgent OR later'
  AND COALESCE(users.age, 0) = 0
ORDER BY 2, 3
LIMIT 10;

-- The delayed clause names both sides. Above the join it reduces to a filter on
-- `products.age`, which rejects product 1.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND orders.note @@@ 'urgent OR later'
  AND COALESCE(users.age, products.age) > 3
ORDER BY 2, 3
LIMIT 10;

SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND orders.note @@@ 'urgent OR later'
  AND COALESCE(users.age, products.age) > 3
ORDER BY 2, 3
LIMIT 10;

-- A search predicate in the delayed clause, with its other arm on the pruned side.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND ((orders.note @@@ 'urgent') OR (users.name @@@ 'alice'))
ORDER BY 2, 3
LIMIT 10;

SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND ((orders.note @@@ 'urgent') OR (users.name @@@ 'alice'))
ORDER BY 2, 3
LIMIT 10;

-- The Anti join planned as a sub-join, so its delayed clause is absorbed during
-- path reconstruction instead of at the top level. The clause rejects product 1,
-- so a dropped clause would show up as an extra row.
SET join_collapse_limit = 1;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT users.id, products.id, orders.id
FROM (products LEFT JOIN users ON users.name = products.name AND users.age >= products.age)
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND orders.note @@@ 'urgent OR later'
  AND COALESCE(users.age, products.age) > 3
ORDER BY 2, 3
LIMIT 10;

SELECT users.id, products.id, orders.id
FROM (products LEFT JOIN users ON users.name = products.name AND users.age >= products.age)
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND orders.note @@@ 'urgent OR later'
  AND COALESCE(users.age, products.age) > 3
ORDER BY 2, 3
LIMIT 10;

RESET join_collapse_limit;

-- A `NOT EXISTS` anti join keeps its own conditions in the join.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT products.id, orders.id
FROM products
JOIN orders ON products.id = orders.id
WHERE orders.note @@@ 'urgent OR later'
  AND NOT EXISTS (
    SELECT 1 FROM users
    WHERE users.name = products.name AND users.age >= products.age
  )
ORDER BY 1, 2
LIMIT 10;

SELECT products.id, orders.id
FROM products
JOIN orders ON products.id = orders.id
WHERE orders.note @@@ 'urgent OR later'
  AND NOT EXISTS (
    SELECT 1 FROM users
    WHERE users.name = products.name AND users.age >= products.age
  )
ORDER BY 1, 2
LIMIT 10;

DROP TABLE orders CASCADE;
DROP TABLE products CASCADE;
DROP TABLE users CASCADE;

\i common/common_cleanup.sql
