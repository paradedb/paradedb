CREATE EXTENSION IF NOT EXISTS pg_search;

-- This test demonstrates an extremely expensive nested-loop join plan
-- as described in https://github.com/paradedb/paradedb/issues/2733.
-- TODO: The test cannot be scaled up to 10k rows per-table as shown
-- in the original repro because the nested-loop plan runs in exponential time.
-- We keep a minimal amount of data to reproduce the plan structure.

DROP TABLE IF EXISTS users CASCADE;
CREATE TABLE users (id SERIAL8 PRIMARY KEY, name TEXT, color VARCHAR, age VARCHAR);
CREATE INDEX idxusers ON users USING bm25 (id, name, color, age) WITH (key_field = 'id', text_fields = '{ "name": { "tokenizer": { "type": "keyword" }, "fast": true }, "color": { "tokenizer": { "type": "keyword" }, "fast": true } }');
INSERT INTO users (name, color, age) VALUES ('bob', 'blue', '20');
ANALYZE;

DROP TABLE IF EXISTS products CASCADE;
CREATE TABLE products (id SERIAL8 PRIMARY KEY, name TEXT, color VARCHAR);
CREATE INDEX idxproducts ON products USING bm25 (id, name, color) WITH (key_field = 'id', text_fields = '{ "name": { "tokenizer": { "type": "keyword" }, "fast": true }, "color": { "tokenizer": { "type": "keyword" }, "fast": true } }');
INSERT INTO products (name, color) VALUES ('bob', 'blue');
ANALYZE;

DROP TABLE IF EXISTS orders CASCADE;
CREATE TABLE orders (id SERIAL8 PRIMARY KEY, name TEXT, color VARCHAR);
CREATE INDEX idxorders ON orders USING bm25 (id, name, color) WITH (key_field = 'id', text_fields = '{ "name": { "tokenizer": { "type": "keyword" }, "fast": true }, "color": { "tokenizer": { "type": "keyword" }, "fast": true } }');
INSERT INTO orders (name, color) VALUES ('bob', 'blue');
ANALYZE;

-- When running under the joinscan, we get a `pdb_search_predicate` join filter.
SET paradedb.enable_join_custom_scan = on;
SET work_mem TO '64MB';

EXPLAIN
SELECT users.name, users.color, users.age
FROM users JOIN products ON users.name = products.name JOIN orders ON products.color = orders.color
WHERE ((orders.id @@@ '3') AND (orders.color @@@ 'blue')) OR ((users.color @@@ 'blue') AND (users.id @@@ '3'))
LIMIT 10;

SELECT users.name, users.color, users.age
FROM users JOIN products ON users.name = products.name JOIN orders ON products.color = orders.color
WHERE ((orders.id @@@ '3') AND (orders.color @@@ 'blue')) OR ((users.color @@@ 'blue') AND (users.id @@@ '3'))
LIMIT 10;

-- Cleanup
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS orders CASCADE;
