CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS orders;

CREATE TABLE users (id SERIAL8 NOT NULL PRIMARY KEY, age INTEGER);
CREATE INDEX idxusers ON users USING bm25 (id, age) WITH (
    key_field = 'id',
    numeric_fields = '{ "age": { "fast": true } }'
);
INSERT INTO users (id, age) VALUES (1, 20);
INSERT INTO users (id, age) VALUES (4, 98);

CREATE INDEX idxusers_id ON users (id);
CREATE INDEX idxusers_age ON users (age);
ANALYZE users;

CREATE TABLE products (id SERIAL8 NOT NULL PRIMARY KEY, age INTEGER);
CREATE INDEX idxproducts ON products USING bm25 (id, age) WITH (
    key_field = 'id',
    numeric_fields = '{ "age": { "fast": true } }'
);
INSERT INTO products (id, age) VALUES (1, 20);
INSERT INTO products (id, age) VALUES (4, 75);
INSERT INTO products (id, age) VALUES (11, 69);

CREATE INDEX idxproducts_id ON products (id);
CREATE INDEX idxproducts_age ON products (age);
ANALYZE products;

CREATE TABLE orders (id SERIAL8 NOT NULL PRIMARY KEY, age INTEGER);
CREATE INDEX idxorders ON orders USING bm25 (id, age) WITH (
    key_field = 'id',
    numeric_fields = '{ "age": { "fast": true } }'
);
INSERT INTO orders (id, age) VALUES (1, 20);
INSERT INTO orders (id, age) VALUES (4, 51);
INSERT INTO orders (id, age) VALUES (5, 69);
INSERT INTO orders (id, age) VALUES (7, 69);

CREATE INDEX idxorders_id ON orders (id);
CREATE INDEX idxorders_age ON orders (age);
ANALYZE orders;

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

-- PostgreSQL query:
SELECT COUNT(*) FROM users RIGHT JOIN products ON users.id = products.id JOIN orders ON products.age = orders.age  WHERE ((orders.id  =  '4') OR (products.id  =  '4')) AND ((users.id  =  '4') OR (NOT (products.id  =  '4')));

-- Set GUCs to match the failing test case
SET paradedb.enable_aggregate_custom_scan TO true;
SET paradedb.enable_custom_scan TO true;
SET paradedb.enable_custom_scan_without_operator TO false;
SET paradedb.enable_filter_pushdown TO true;
SET paradedb.enable_join_custom_scan TO false;
SET enable_seqscan TO false;
SET enable_indexscan TO false;
SET max_parallel_workers TO 8;
SET max_parallel_workers_per_gather TO 4;
SET parallel_leader_participation TO false;
SET paradedb.add_doc_count_to_aggs TO true;
SET paradedb.enable_columnar_exec TO true;
SET paradedb.min_rows_per_worker TO 10;
SET statement_timeout TO 60000;

-- BM25 query:
SELECT COUNT(*) FROM users RIGHT JOIN products ON users.id = products.id JOIN orders ON products.age = orders.age  WHERE ((orders.id @@@ '4') OR (products.id @@@ '4')) AND ((users.id @@@ '4') OR (NOT (products.id @@@ '4')));

DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS orders;
