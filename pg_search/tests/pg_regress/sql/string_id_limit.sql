-- Setup
\i common/common_setup.sql

DROP TABLE IF EXISTS comments CASCADE;
-- create a table with an id, customer_id, text, a created_at timestamp.
CREATE TABLE comments (
	id VARCHAR(24) PRIMARY KEY,
	customer_id VARCHAR(64) NOT NULL,
	text TEXT DEFAULT ''
);
-- add search index
CREATE INDEX comments_search ON comments USING bm25(
	id, customer_id, text
)
WITH (
	key_field='id',
	text_fields='{
		"customer_id": {
			"fast":true,
			"tokenizer": {"type": "keyword"},
			"record": "basic"
		}
	}'
);
-- populate the table with faulty data
INSERT INTO comments (id, customer_id) VALUES ('ctx_01ifsur2egUPnbJOAiHv', 'customer_1');
INSERT INTO comments (id, customer_id) VALUES ('ctx_01iddo3tioqV6f4yCB6O', 'customer_1');
INSERT INTO comments (id, customer_id) VALUES ('ctx_01ic75tgb5J5XkhJqkjn', 'customer_1');

-- this row should be the first one returned
INSERT INTO comments (id, customer_id) VALUES ('ctx_01iso5q4prkOQVGKK0ue', 'customer_1');
-- Select w limit. This will return
-- "ctx_01ifsur2egUPnbJOAiHv" first despite it being second in the order
SELECT * FROM comments
WHERE id @@@ paradedb.term('customer_id', 'customer_1')
ORDER BY id desc
LIMIT 2;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM comments
WHERE id @@@ paradedb.term('customer_id', 'customer_1')
ORDER BY id desc
LIMIT 2;

-- Select w/o limit, returns correctly ordered results,
-- "ctx_01iso5q4prkOQVGKK0ue" first
SELECT * FROM comments
WHERE id @@@ paradedb.term('customer_id', 'customer_1')
ORDER BY id DESC;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM comments
WHERE id @@@ paradedb.term('customer_id', 'customer_1')
ORDER BY id DESC;
