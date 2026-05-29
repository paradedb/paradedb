-- Regression test: JoinScan declines on a 3-way INNER join when PG picks a
-- parameterized `NestPath` for the inner sub-join.
--
-- This is a JoinScan-activation gap, not a correctness gap. PG's fallback
-- nested-loop / bitmap plan returns the right rows; JoinScan just never
-- fires, so the query loses the BM25 join acceleration.
--
-- With `enable_seqscan` and `enable_indexscan` off, PG plans the
-- `(users JOIN products)` sub-join as a `NestPath` whose inner side is a
-- parameterized bitmap-index lookup on `idxproducts_age`. The join clause
-- `users.age = products.age` is enforced via that index condition, so PG
-- leaves `JoinPath.joinrestrictinfo` empty. `collect_join_sources_join_rel`
-- reads only `joinrestrictinfo` to recover equi-keys, finds nothing,
-- returns `None`, and the 3-way JoinScan never gets registered.

SET max_parallel_workers = 0;
SET max_parallel_workers_per_gather = 0;
SET parallel_leader_participation = off;

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS orders CASCADE;

CREATE TABLE users (id BIGSERIAL PRIMARY KEY, name TEXT, age INTEGER, uuid UUID);
INSERT INTO users (name, age, uuid)
SELECT (ARRAY['alice','bob','cloe']::text[])[((i % 3) + 1)],
       ((i % 100) + 1),
       md5(i::text)::uuid
FROM generate_series(1, 100) AS i;
CREATE INDEX idxusers ON users USING bm25 (id, name, age, uuid)
  WITH (key_field = 'id',
        text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true},
                        "uuid": {"tokenizer": {"type": "keyword"}, "fast": true}}',
        numeric_fields = '{"age": {"fast": true}}');
CREATE INDEX idxusers_age ON users (age);
CREATE INDEX idxusers_uuid ON users (uuid);
ANALYZE users;

CREATE TABLE products (id BIGSERIAL PRIMARY KEY, name TEXT, age INTEGER, uuid UUID);
INSERT INTO products (name, age, uuid) SELECT name, age, uuid FROM users;
CREATE INDEX idxproducts ON products USING bm25 (id, name, age, uuid)
  WITH (key_field = 'id',
        text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true},
                        "uuid": {"tokenizer": {"type": "keyword"}, "fast": true}}',
        numeric_fields = '{"age": {"fast": true}}');
CREATE INDEX idxproducts_age ON products (age);
CREATE INDEX idxproducts_uuid ON products (uuid);
ANALYZE products;

CREATE TABLE orders (id BIGSERIAL PRIMARY KEY, name TEXT, age INTEGER, uuid UUID);
INSERT INTO orders (name, age, uuid) SELECT name, age, uuid FROM users;
CREATE INDEX idxorders ON orders USING bm25 (id, name, age, uuid)
  WITH (key_field = 'id',
        text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true},
                        "uuid": {"tokenizer": {"type": "keyword"}, "fast": true}}',
        numeric_fields = '{"age": {"fast": true}}');
CREATE INDEX idxorders_age ON orders (age);
CREATE INDEX idxorders_uuid ON orders (uuid);
ANALYZE orders;

SET paradedb.enable_aggregate_custom_scan = off;
SET paradedb.enable_custom_scan = off;
SET paradedb.enable_join_custom_scan = on;
SET enable_seqscan = off;
SET enable_indexscan = off;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT users.id, users.name
FROM users
JOIN products ON users.age = products.age
JOIN orders ON products.uuid = orders.uuid
WHERE users.name @@@ 'bob'
ORDER BY users.id, products.id, orders.id LIMIT 1;

SELECT users.id, users.name
FROM users
JOIN products ON users.age = products.age
JOIN orders ON products.uuid = orders.uuid
WHERE users.name @@@ 'bob'
ORDER BY users.id, products.id, orders.id LIMIT 1;

DROP TABLE users CASCADE;
DROP TABLE products CASCADE;
DROP TABLE orders CASCADE;
