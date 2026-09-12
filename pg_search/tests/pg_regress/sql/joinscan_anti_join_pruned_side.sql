-- A `LEFT JOIN` whose `ON` clause is strict in a column that `WHERE` tests with
-- `IS NULL` becomes an Anti join, and the inner side leaves the join output.
-- Postgres still evaluates a `WHERE` clause that names that side above the join,
-- where its columns are null-extended. The JoinScan must see the same NULL there
-- instead of asking the pruned scan for a column or a match tag.

\i common/common_setup.sql

DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS categories CASCADE;

CREATE TABLE users      (id SERIAL8 PRIMARY KEY, name TEXT, age INTEGER);
CREATE TABLE products   (id SERIAL8 PRIMARY KEY, name TEXT, age INTEGER);
CREATE TABLE orders     (id SERIAL8 PRIMARY KEY, age INTEGER, note TEXT);
CREATE TABLE categories (id SERIAL8 PRIMARY KEY, label TEXT);

CREATE INDEX idxusers ON users USING bm25 (id, name, age)
  WITH (key_field='id', text_fields='{"name":{"fast":true}}', numeric_fields='{"age":{"fast":true}}');
CREATE INDEX idxproducts ON products USING bm25 (id, name, age)
  WITH (key_field='id', text_fields='{"name":{"fast":true}}', numeric_fields='{"age":{"fast":true}}');
CREATE INDEX idxorders ON orders USING bm25 (id, age, note)
  WITH (key_field='id', text_fields='{"note":{"fast":true}}', numeric_fields='{"age":{"fast":true}}');
CREATE INDEX idxcategories ON categories USING bm25 (id, label)
  WITH (key_field='id', text_fields='{"label":{"fast":true}}');

-- `alice` has a NULL age, so `users.age >= products.age` never matches her and
-- product 1 survives the Anti join. `bob` matches product 2, which drops it.
INSERT INTO users (name, age)      VALUES ('alice', NULL), ('bob', 5), ('carol', 1);
INSERT INTO products (name, age)   VALUES ('alice', 3), ('bob', 4), ('carol', 9), ('dave', 2);
INSERT INTO orders (age, note)     VALUES (7, 'urgent'), (8, 'later'), (10, 'urgent later'), (1, 'later');
INSERT INTO categories (label)     VALUES ('food'), ('toys'), ('books'), ('games');

SET paradedb.enable_join_custom_scan TO on;

-- The reported shape: two `FULL JOIN`s that the planner reduces to an Anti join
-- under an inner join.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT users.id, products.id, orders.id
FROM users
FULL JOIN products ON users.name = products.name AND users.age >= products.age
FULL JOIN orders ON products.id = orders.id
WHERE ((orders.note @@@ 'urgent') OR (users.name @@@ 'alice'))
  AND users.age IS NULL
  AND products.age <= orders.age
ORDER BY 2, 3
LIMIT 10;

SELECT users.id, products.id, orders.id
FROM users
FULL JOIN products ON users.name = products.name AND users.age >= products.age
FULL JOIN orders ON products.id = orders.id
WHERE ((orders.note @@@ 'urgent') OR (users.name @@@ 'alice'))
  AND users.age IS NULL
  AND products.age <= orders.age
ORDER BY 2, 3
LIMIT 10;

-- The pruned arm is NULL, so the `OR` only keeps rows the other arm matches.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND ((orders.note @@@ 'later') OR (users.name @@@ 'alice'))
ORDER BY 2, 3
LIMIT 10;

SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND ((orders.note @@@ 'later') OR (users.name @@@ 'alice'))
ORDER BY 2, 3
LIMIT 10;

-- `NOT (FALSE OR NULL)` is NULL, not TRUE: no row may come back.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND NOT ((orders.note @@@ 'later') OR (users.name @@@ 'alice'))
ORDER BY 2, 3
LIMIT 10;

SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND NOT ((orders.note @@@ 'later') OR (users.name @@@ 'alice'))
ORDER BY 2, 3
LIMIT 10;

-- A plain comparison on the pruned side is NULL as well.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND ((orders.note @@@ 'later') OR (users.age > 0))
ORDER BY 2, 3
LIMIT 10;

SELECT users.id, products.id, orders.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND ((orders.note @@@ 'later') OR (users.age > 0))
ORDER BY 2, 3
LIMIT 10;

-- The Anti join sits two levels down, under another inner join.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT users.id, products.id, orders.id, categories.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
JOIN categories ON categories.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND ((orders.note @@@ 'urgent') OR (users.name @@@ 'alice'))
  AND categories.label @@@ 'food OR books'
ORDER BY 2, 3, 4
LIMIT 10;

SELECT users.id, products.id, orders.id, categories.id
FROM products
LEFT JOIN users ON users.name = products.name AND users.age >= products.age
JOIN orders ON products.id = orders.id
JOIN categories ON categories.id = orders.id
WHERE users.age IS NULL
  AND products.age <= orders.age
  AND ((orders.note @@@ 'urgent') OR (users.name @@@ 'alice'))
  AND categories.label @@@ 'food OR books'
ORDER BY 2, 3, 4
LIMIT 10;

DROP TABLE categories CASCADE;
DROP TABLE orders CASCADE;
DROP TABLE products CASCADE;
DROP TABLE users CASCADE;

\i common/common_cleanup.sql
