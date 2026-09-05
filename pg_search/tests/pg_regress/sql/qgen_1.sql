-- ==== RESULT MISMATCH REPRODUCTION SCRIPT ====
-- Copy and paste this entire block to reproduce the issue
--
-- Prerequisites: Ensure pg_search extension is available
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;
--
-- Table and index setup
SET seed TO -0.6613270148901305;
-- PARADEDB_QGEN_SEED: 7538094396191304247
-- qgen bulk inserts: 1

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
category TEXT, 
literal_normalized TEXT, 
metadata JSONB, 
tags TEXT[]
);
-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idxusers ON users USING paradedb (id, uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, (upper(category)::pdb.literal), (literal_normalized::pdb.literal_normalized), metadata, tags) WITH (
    key_field = 'id',
    text_fields = '{ "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
"name": { "tokenizer": { "type": "keyword" }, "fast": true },
"color": { "tokenizer": { "type": "keyword" }, "fast": true },
"tags": { "tokenizer": { "type": "keyword" }, "fast": true } }',
    numeric_fields = '{ "age": { "fast": true },
"quantity": { "fast": true },
"price": { "fast": true },
"small_numeric": { "fast": true },
"int_numeric": { "fast": true },
"high_scale": { "fast": true },
"big_numeric": { "fast": true } }',
    json_fields = '{ "metadata": { "fast": true } }',
    sort_by = 'age DESC NULLS LAST',
    target_segment_count = 2
);

INSERT into users (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, category, literal_normalized, metadata, tags) VALUES ('550e8400-e29b-41d4-a716-446655440000', 'bob', 'blue', '20', '7', '99.99', '12.34', '12345', '123.456789', '12345.67890', '4', 'electronics', 'Hello World', '{"brand": "apple", "rating": 4}', ARRAY['alpha', 'beta']::text[]);

INSERT into users (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, category, literal_normalized, metadata, tags) SELECT rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid,
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
      (ARRAY ['electronics', 'clothing', 'food', 'books', 'toys', 'sports', 'home']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['Hello World', 'HELLO WORLD', 'hello world', 'HeLLo WoRLD', 'GOODBYE WORLD', 'goodbye world']::text[])[(floor(random() * 6) + 1)::int],
      jsonb_build_object(
                'brand', (ARRAY ['apple', 'samsung', 'sony', 'lg']::text[])[(floor(random() * 4) + 1)::int],
                'rating', (floor(random() * 5) + 1)::int
            ),
      (CASE (floor(random() * 5) + 1)::int WHEN 1 THEN ARRAY['alpha', 'beta']::text[] WHEN 2 THEN ARRAY['gamma']::text[] WHEN 3 THEN ARRAY['delta', 'epsilon', 'zeta']::text[] WHEN 4 THEN NULL ELSE ARRAY[]::text[] END) FROM generate_series(1, 10);

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
CREATE INDEX idxusers_category ON users (category);
CREATE INDEX idxusers_literal_normalized ON users (literal_normalized);
CREATE INDEX idxusers_metadata ON users (metadata);
CREATE INDEX idxusers_tags ON users (tags);

ANALYZE users;

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
category TEXT, 
literal_normalized TEXT, 
metadata JSONB, 
tags TEXT[]
);
-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idxproducts ON products USING paradedb (id, uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, (upper(category)::pdb.literal), (literal_normalized::pdb.literal_normalized), metadata, tags) WITH (
    key_field = 'id',
    text_fields = '{ "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
"name": { "tokenizer": { "type": "keyword" }, "fast": true },
"color": { "tokenizer": { "type": "keyword" }, "fast": true },
"tags": { "tokenizer": { "type": "keyword" }, "fast": true } }',
    numeric_fields = '{ "age": { "fast": true },
"quantity": { "fast": true },
"price": { "fast": true },
"small_numeric": { "fast": true },
"int_numeric": { "fast": true },
"high_scale": { "fast": true },
"big_numeric": { "fast": true } }',
    json_fields = '{ "metadata": { "fast": true } }',
    sort_by = 'age DESC NULLS LAST',
    target_segment_count = 2
);

INSERT into products (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, category, literal_normalized, metadata, tags) VALUES ('550e8400-e29b-41d4-a716-446655440000', 'bob', 'blue', '20', '7', '99.99', '12.34', '12345', '123.456789', '12345.67890', '4', 'electronics', 'Hello World', '{"brand": "apple", "rating": 4}', ARRAY['alpha', 'beta']::text[]);

INSERT into products (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, category, literal_normalized, metadata, tags) SELECT rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid,
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
      (ARRAY ['electronics', 'clothing', 'food', 'books', 'toys', 'sports', 'home']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['Hello World', 'HELLO WORLD', 'hello world', 'HeLLo WoRLD', 'GOODBYE WORLD', 'goodbye world']::text[])[(floor(random() * 6) + 1)::int],
      jsonb_build_object(
                'brand', (ARRAY ['apple', 'samsung', 'sony', 'lg']::text[])[(floor(random() * 4) + 1)::int],
                'rating', (floor(random() * 5) + 1)::int
            ),
      (CASE (floor(random() * 5) + 1)::int WHEN 1 THEN ARRAY['alpha', 'beta']::text[] WHEN 2 THEN ARRAY['gamma']::text[] WHEN 3 THEN ARRAY['delta', 'epsilon', 'zeta']::text[] WHEN 4 THEN NULL ELSE ARRAY[]::text[] END) FROM generate_series(1, 10);

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
CREATE INDEX idxproducts_category ON products (category);
CREATE INDEX idxproducts_literal_normalized ON products (literal_normalized);
CREATE INDEX idxproducts_metadata ON products (metadata);
CREATE INDEX idxproducts_tags ON products (tags);

ANALYZE products;

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
category TEXT, 
literal_normalized TEXT, 
metadata JSONB, 
tags TEXT[]
);
-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idxorders ON orders USING paradedb (id, uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, (upper(category)::pdb.literal), (literal_normalized::pdb.literal_normalized), metadata, tags) WITH (
    key_field = 'id',
    text_fields = '{ "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
"name": { "tokenizer": { "type": "keyword" }, "fast": true },
"color": { "tokenizer": { "type": "keyword" }, "fast": true },
"tags": { "tokenizer": { "type": "keyword" }, "fast": true } }',
    numeric_fields = '{ "age": { "fast": true },
"quantity": { "fast": true },
"price": { "fast": true },
"small_numeric": { "fast": true },
"int_numeric": { "fast": true },
"high_scale": { "fast": true },
"big_numeric": { "fast": true } }',
    json_fields = '{ "metadata": { "fast": true } }',
    sort_by = 'age DESC NULLS LAST',
    target_segment_count = 2
);

INSERT into orders (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, category, literal_normalized, metadata, tags) VALUES ('550e8400-e29b-41d4-a716-446655440000', 'bob', 'blue', '20', '7', '99.99', '12.34', '12345', '123.456789', '12345.67890', '4', 'electronics', 'Hello World', '{"brand": "apple", "rating": 4}', ARRAY['alpha', 'beta']::text[]);

INSERT into orders (uuid, name, color, age, quantity, price, small_numeric, int_numeric, high_scale, big_numeric, rating, category, literal_normalized, metadata, tags) SELECT rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid,
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
      (ARRAY ['electronics', 'clothing', 'food', 'books', 'toys', 'sports', 'home']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['Hello World', 'HELLO WORLD', 'hello world', 'HeLLo WoRLD', 'GOODBYE WORLD', 'goodbye world']::text[])[(floor(random() * 6) + 1)::int],
      jsonb_build_object(
                'brand', (ARRAY ['apple', 'samsung', 'sony', 'lg']::text[])[(floor(random() * 4) + 1)::int],
                'rating', (floor(random() * 5) + 1)::int
            ),
      (CASE (floor(random() * 5) + 1)::int WHEN 1 THEN ARRAY['alpha', 'beta']::text[] WHEN 2 THEN ARRAY['gamma']::text[] WHEN 3 THEN ARRAY['delta', 'epsilon', 'zeta']::text[] WHEN 4 THEN NULL ELSE ARRAY[]::text[] END) FROM generate_series(1, 10);

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
CREATE INDEX idxorders_category ON orders (category);
CREATE INDEX idxorders_literal_normalized ON orders (literal_normalized);
CREATE INDEX idxorders_metadata ON orders (metadata);
CREATE INDEX idxorders_tags ON orders (tags);

ANALYZE orders;
DELETE FROM users WHERE random() < 0.01;
DELETE FROM products WHERE random() < 0.01;
DELETE FROM orders WHERE random() < 0.01;

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
SET max_parallel_workers_per_gather TO 4;
SET parallel_leader_participation TO true;
SET paradedb.add_doc_count_to_aggs TO true;
SET paradedb.enable_columnar_exec TO false;
SET paradedb.min_rows_per_worker TO 10;
SET statement_timeout TO 60000;

--
-- PostgreSQL query:
EXPLAIN
SELECT DISTINCT users.id, users.name, products.id, orders.id, products.color IS NULL FROM users JOIN products ON users.name = products.name RIGHT JOIN orders ON products.name = orders.name AND products.age >= orders.age WHERE ((users.name  =  'bob') AND (products.name  =  'bob')) AND (users.age > orders.age) ORDER BY users.id, products.id, orders.id LIMIT 22;
SELECT DISTINCT users.id, users.name, products.id, orders.id, products.color IS NULL FROM users JOIN products ON users.name = products.name RIGHT JOIN orders ON products.name = orders.name AND products.age >= orders.age WHERE ((users.name  =  'bob') AND (products.name  =  'bob')) AND (users.age > orders.age) ORDER BY users.id, products.id, orders.id LIMIT 22;
--
-- Set GUCs to match the failing test case
SET paradedb.enable_aggregate_custom_scan TO false;
SET paradedb.enable_custom_scan TO true;
SET paradedb.enable_custom_scan_without_operator TO true;
SET paradedb.enable_filter_pushdown TO true;
SET paradedb.enable_join_custom_scan TO true;
SET enable_seqscan TO false;
SET enable_indexscan TO false;
SET max_parallel_workers TO 0;
SET max_parallel_workers_per_gather TO 0;
SET parallel_leader_participation TO true;
SET paradedb.add_doc_count_to_aggs TO true;
SET paradedb.enable_columnar_exec TO true;
RESET paradedb.min_rows_per_worker;
SET statement_timeout TO 60000;

--
-- ParadeDB query:
EXPLAIN
SELECT DISTINCT users.id, users.name, products.id, orders.id, products.color IS NULL FROM users JOIN products ON users.name = products.name RIGHT JOIN orders ON products.name = orders.name AND products.age >= orders.age WHERE ((users.name @@@ 'bob') AND (products.name @@@ 'bob')) AND (users.age > orders.age) ORDER BY users.id, products.id, orders.id LIMIT 22;
SELECT DISTINCT users.id, users.name, products.id, orders.id, products.color IS NULL FROM users JOIN products ON users.name = products.name RIGHT JOIN orders ON products.name = orders.name AND products.age >= orders.age WHERE ((users.name @@@ 'bob') AND (products.name @@@ 'bob')) AND (users.age > orders.age) ORDER BY users.id, products.id, orders.id LIMIT 22;
--
-- Cleanup:
DROP TABLE users;
DROP TABLE products;
DROP TABLE orders;
--
-- ==== END REPRODUCTION SCRIPT ====
