CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;

--
-- Table and index setup
SET seed TO -0.39666141937798427;

CREATE TABLE users (
    id SERIAL8 NOT NULL PRIMARY KEY,
uuid UUID,
name TEXT,
color VARCHAR,
age INTEGER,
quantity INTEGER,
price NUMERIC(10,2),
small_numeric NUMERIC(5,2),
int_numeric NUMERIC(10,0),
high_scale NUMERIC(18,6),
big_numeric NUMERIC,
rating INTEGER,
literal_normalized TEXT
);
-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idxusers ON users USING bm25 (id, uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, (literal_normalized::pdb.literal_normalized)) WITH (
    key_field = 'id',
    text_fields = '{ "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
"name": { "tokenizer": { "type": "keyword" }, "fast": true },
"color": { "tokenizer": { "type": "keyword" }, "fast": true } }',
    numeric_fields = '{ "age": { "fast": true },
"quantity": { "fast": true },
"price": { "fast": true },
"small_numeric": { "fast": true },
"int_numeric": { "fast": true },
"high_scale": { "fast": true },
"big_numeric": { "fast": true } }',
    sort_by = 'age DESC NULLS LAST'
);

INSERT into users (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, literal_normalized) VALUES ('550e8400-e29b-41d4-a716-446655440000', 'bob', 'blue', '20', '7', '99.99', '12.34', '12345', '123.456789', '12345.67890', '4', 'Hello World');

INSERT into users (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, literal_normalized) SELECT rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid,
      (ARRAY ['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow', NULL]::text[])[(floor(random() * 8) + 1)::int],
      (floor(random() * 100) + 1),
      CASE WHEN random() < 0.1 THEN NULL ELSE (floor(random() * 100) + 1)::int END,
      (random() * 1000 + 10)::numeric(10,2),
      (random() * 100)::numeric(5,2),
      (floor(random() * 1000000))::numeric(10,0),
      (random() * 10000)::numeric(18,6),
      (random() * 100000)::numeric,
      (floor(random() * 5) + 1)::int,
      (ARRAY ['Hello World', 'HELLO WORLD', 'hello world', 'HeLLo WoRLD', 'GOODBYE WORLD', 'goodbye world']::text[])[(floor(random() * 6) + 1)::int] FROM generate_series(1, 100);

CREATE INDEX idxusers_id ON users (id);
CREATE INDEX idxusers_uuid ON users (uuid);
CREATE INDEX idxusers_name ON users (name);
CREATE INDEX idxusers_color ON users (color);
CREATE INDEX idxusers_age ON users (age);
CREATE INDEX idxusers_quantity ON users (quantity);
CREATE INDEX idxusers_price ON users (price);
CREATE INDEX idxusers_small_numeric ON users (small_numeric);
CREATE INDEX idxusers_int_numeric ON users (int_numeric);
CREATE INDEX idxusers_high_scale ON users (high_scale);
CREATE INDEX idxusers_big_numeric ON users (big_numeric);
CREATE INDEX idxusers_literal_normalized ON users (literal_normalized);

ANALYZE;

CREATE TABLE products (
    id SERIAL8 NOT NULL PRIMARY KEY,
uuid UUID,
name TEXT,
color VARCHAR,
age INTEGER,
quantity INTEGER,
price NUMERIC(10,2),
small_numeric NUMERIC(5,2),
int_numeric NUMERIC(10,0),
high_scale NUMERIC(18,6),
big_numeric NUMERIC,
rating INTEGER,
literal_normalized TEXT
);
-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idxproducts ON products USING bm25 (id, uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, (literal_normalized::pdb.literal_normalized)) WITH (
    key_field = 'id',
    text_fields = '{ "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
"name": { "tokenizer": { "type": "keyword" }, "fast": true },
"color": { "tokenizer": { "type": "keyword" }, "fast": true } }',
    numeric_fields = '{ "age": { "fast": true },
"quantity": { "fast": true },
"price": { "fast": true },
"small_numeric": { "fast": true },
"int_numeric": { "fast": true },
"high_scale": { "fast": true },
"big_numeric": { "fast": true } }',
    sort_by = 'age DESC NULLS LAST'
);

INSERT into products (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, literal_normalized) VALUES ('550e8400-e29b-41d4-a716-446655440000', 'bob', 'blue', '20', '7', '99.99', '12.34', '12345', '123.456789', '12345.67890', '4', 'Hello World');

INSERT into products (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, literal_normalized) SELECT rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid,
      (ARRAY ['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow', NULL]::text[])[(floor(random() * 8) + 1)::int],
      (floor(random() * 100) + 1),
      CASE WHEN random() < 0.1 THEN NULL ELSE (floor(random() * 100) + 1)::int END,
      (random() * 1000 + 10)::numeric(10,2),
      (random() * 100)::numeric(5,2),
      (floor(random() * 1000000))::numeric(10,0),
      (random() * 10000)::numeric(18,6),
      (random() * 100000)::numeric,
      (floor(random() * 5) + 1)::int,
      (ARRAY ['Hello World', 'HELLO WORLD', 'hello world', 'HeLLo WoRLD', 'GOODBYE WORLD', 'goodbye world']::text[])[(floor(random() * 6) + 1)::int] FROM generate_series(1, 100);

CREATE INDEX idxproducts_id ON products (id);
CREATE INDEX idxproducts_uuid ON products (uuid);
CREATE INDEX idxproducts_name ON products (name);

CREATE INDEX idxproducts_color ON products (color);
CREATE INDEX idxproducts_age ON products (age);
CREATE INDEX idxproducts_quantity ON products (quantity);
CREATE INDEX idxproducts_price ON products (price);
CREATE INDEX idxproducts_small_numeric ON products (small_numeric);
CREATE INDEX idxproducts_int_numeric ON products (int_numeric);
CREATE INDEX idxproducts_high_scale ON products (high_scale);
CREATE INDEX idxproducts_big_numeric ON products (big_numeric);
CREATE INDEX idxproducts_literal_normalized ON products (literal_normalized);

ANALYZE;

CREATE TABLE orders (
    id SERIAL8 NOT NULL PRIMARY KEY,
uuid UUID,
name TEXT,
color VARCHAR,
age INTEGER,
quantity INTEGER,
price NUMERIC(10,2),
small_numeric NUMERIC(5,2),
int_numeric NUMERIC(10,0),
high_scale NUMERIC(18,6),
big_numeric NUMERIC,
rating INTEGER,
literal_normalized TEXT
);
-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idxorders ON orders USING bm25 (id, uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, (literal_normalized::pdb.literal_normalized)) WITH (
    key_field = 'id',
    text_fields = '{ "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
"name": { "tokenizer": { "type": "keyword" }, "fast": true },
"color": { "tokenizer": { "type": "keyword" }, "fast": true } }',
    numeric_fields = '{ "age": { "fast": true },
"quantity": { "fast": true },
"price": { "fast": true },
"small_numeric": { "fast": true },
"int_numeric": { "fast": true },
"high_scale": { "fast": true },
"big_numeric": { "fast": true } }',
    sort_by = 'age DESC NULLS LAST'
);

INSERT into orders (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, literal_normalized) VALUES ('550e8400-e29b-41d4-a716-446655440000', 'bob', 'blue', '20', '7', '99.99', '12.34', '12345', '123.456789', '12345.67890', '4', 'Hello World');

INSERT into orders (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, literal_normalized) SELECT rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid,
      (ARRAY ['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow', NULL]::text[])[(floor(random() * 8) + 1)::int],
      (floor(random() * 100) + 1),
      CASE WHEN random() < 0.1 THEN NULL ELSE (floor(random() * 100) + 1)::int END,
      (random() * 1000 + 10)::numeric(10,2),
      (random() * 100)::numeric(5,2),
      (floor(random() * 1000000))::numeric(10,0),
      (random() * 10000)::numeric(18,6),
      (random() * 100000)::numeric,
      (floor(random() * 5) + 1)::int,
      (ARRAY ['Hello World', 'HELLO WORLD', 'hello world', 'HeLLo WoRLD', 'GOODBYE WORLD', 'goodbye world']::text[])[(floor(random() * 6) + 1)::int] FROM generate_series(1, 100);

CREATE INDEX idxorders_id ON orders (id);
CREATE INDEX idxorders_uuid ON orders (uuid);
CREATE INDEX idxorders_name ON orders (name);
CREATE INDEX idxorders_color ON orders (color);
CREATE INDEX idxorders_age ON orders (age);
CREATE INDEX idxorders_quantity ON orders (quantity);
CREATE INDEX idxorders_price ON orders (price);
CREATE INDEX idxorders_small_numeric ON orders (small_numeric);
CREATE INDEX idxorders_int_numeric ON orders (int_numeric);
CREATE INDEX idxorders_high_scale ON orders (high_scale);
CREATE INDEX idxorders_big_numeric ON orders (big_numeric);
CREATE INDEX idxorders_literal_normalized ON orders (literal_normalized);

ANALYZE;

--
-- Default GUCs:
SET paradedb.enable_aggregate_custom_scan TO false;
SET paradedb.enable_custom_scan TO false;
SET paradedb.enable_custom_scan_without_operator TO false;
SET paradedb.enable_filter_pushdown TO false;
SET paradedb.enable_join_custom_scan TO false;
SET enable_seqscan TO true;
SET enable_indexscan TO true;
SET max_parallel_workers TO 8;
SET paradedb.add_doc_count_to_aggs TO true;
SET paradedb.enable_mixed_fast_field_exec TO false;
SET paradedb.enable_mixed_fast_field_sort TO true;

--
-- PostgreSQL query:
SELECT users.id, users.name FROM users JOIN products ON users.id = products.id JOIN orders ON products.age = orders.age WHERE (users.name  =  'bob') OR (users.name  =  'bob') ORDER BY users.id, products.id, orders.id LIMIT 29;
--
-- Set GUCs to match the failing test case
SET paradedb.enable_aggregate_custom_scan TO false;
SET paradedb.enable_custom_scan TO false;
SET paradedb.enable_custom_scan_without_operator TO false;
SET paradedb.enable_filter_pushdown TO false;
SET paradedb.enable_join_custom_scan TO true;
SET enable_seqscan TO true;
SET enable_indexscan TO true;
SET max_parallel_workers TO 8;
SET paradedb.add_doc_count_to_aggs TO true;
SET paradedb.enable_mixed_fast_field_exec TO false;
SET paradedb.enable_mixed_fast_field_sort TO true;
SET debug_parallel_query TO on;

--
-- BM25 query:
SELECT users.id, users.name FROM users JOIN products ON users.id = products.id JOIN orders ON products.age = orders.age WHERE (users.name @@@ 'bob') OR (users.name @@@ 'bob') ORDER BY users.id, products.id, orders.id LIMIT 29;
