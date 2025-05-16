-- Tests complex join queries with mixed fields

\i common/mixedff_queries_setup.sql

\echo 'Test: join with mixed fast fields'

DROP TABLE IF EXISTS users CASCADE;
CREATE TABLE users
(
    id    serial8 not null primary key,
    name  text,
    color varchar,
    age   varchar
);

INSERT into users (name, color, age)
VALUES ('bob', 'blue', 20);

-- Use deterministic data patterns instead of random data
INSERT into users (name, color, age)
SELECT
    (ARRAY['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy'])[1 + mod(s.a, 7)],
    (ARRAY['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow'])[1 + mod(s.a, 7)],
    (20 + mod(s.a, 80))::text
FROM generate_series(1, 10000) as s(a);

CREATE INDEX idxusers ON users USING bm25 (id, name, color, age)
WITH (
key_field = 'id',
text_fields = '
            {
                "name": { "tokenizer": { "type": "keyword" }, "fast": true },
                "color": { "tokenizer": { "type": "keyword" }, "fast": true },
                "age": { "tokenizer": { "type": "keyword" }, "fast": true }
            }'
);
CREATE INDEX idxusers_name ON users (name);
CREATE INDEX idxusers_color ON users (color);
CREATE INDEX idxusers_age ON users (age);
ANALYZE;

DROP TABLE IF EXISTS products CASCADE;
CREATE TABLE products
(
    id    serial8 not null primary key,
    name  text,
    color varchar,
    age   varchar
);

INSERT into products (name, color, age)
VALUES ('bob', 'blue', 20);

-- Use deterministic data patterns instead of random data
INSERT into products (name, color, age)
SELECT
    (ARRAY['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy'])[1 + mod(s.a, 7)],
    (ARRAY['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow'])[1 + mod(s.a, 7)],
    (20 + mod(s.a, 80))::text
FROM generate_series(1, 10000) as s(a);

CREATE INDEX idxproducts ON products USING bm25 (id, name, color, age)
WITH (
key_field = 'id',
text_fields = '
            {
                "name": { "tokenizer": { "type": "keyword" }, "fast": true },
                "color": { "tokenizer": { "type": "keyword" }, "fast": true },
                "age": { "tokenizer": { "type": "keyword" }, "fast": true }
            }'
);
CREATE INDEX idxproducts_name ON products (name);
CREATE INDEX idxproducts_color ON products (color);
CREATE INDEX idxproducts_age ON products (age);
ANALYZE;

DROP TABLE IF EXISTS orders CASCADE;
CREATE TABLE orders
(
    id    serial8 not null primary key,
    name  text,
    color varchar,
    age   varchar
);

INSERT into orders (name, color, age)
VALUES ('bob', 'blue', 20);

-- Use deterministic data patterns instead of random data
INSERT into orders (name, color, age)
SELECT
    (ARRAY['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy'])[1 + mod(s.a, 7)],
    (ARRAY['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow'])[1 + mod(s.a, 7)],
    (20 + mod(s.a, 80))::text
FROM generate_series(1, 10000) as s(a);

CREATE INDEX idxorders ON orders USING bm25 (id, name, color, age)
WITH (
key_field = 'id',
text_fields = '
            {
                "name": { "tokenizer": { "type": "keyword" }, "fast": true },
                "color": { "tokenizer": { "type": "keyword" }, "fast": true },
                "age": { "tokenizer": { "type": "keyword" }, "fast": true }
            }'
);
CREATE INDEX idxorders_name ON orders (name);
CREATE INDEX idxorders_color ON orders (color);
CREATE INDEX idxorders_age ON orders (age);
ANALYZE;

SELECT users.color FROM users JOIN orders ON users.id = orders.id  WHERE (users.color  =  'blue') AND (users.name  =  'bob') LIMIT 10;

vacuum;
SET paradedb.enable_mixed_fast_field_exec = false;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT users.color FROM users JOIN orders ON users.id = orders.id  WHERE (users.color @@@ 'blue') AND (users.name @@@ 'bob') LIMIT 10;
SELECT users.color FROM users JOIN orders ON users.id = orders.id  WHERE (users.color @@@ 'blue') AND (users.name @@@ 'bob') LIMIT 10;

SET paradedb.enable_mixed_fast_field_exec = true;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT users.color FROM users JOIN orders ON users.id = orders.id  WHERE (users.color @@@ 'blue') AND (users.name @@@ 'bob') LIMIT 10;
SELECT users.color FROM users JOIN orders ON users.id = orders.id  WHERE (users.color @@@ 'blue') AND (users.name @@@ 'bob') LIMIT 10;


\i common/mixedff_queries_cleanup.sql
