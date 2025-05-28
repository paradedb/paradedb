-- Test various PostgreSQL data types as key_field types in BM25 indexes
-- This test mimics the key.rs test in the Rust test suite

\echo 'Testing different PostgreSQL data types as key fields'

-- Clean up any existing tables/indexes
DROP TABLE IF EXISTS bigint_test;
DROP TABLE IF EXISTS uuid_test;
DROP TABLE IF EXISTS timestamp_test;

-- Test 1: BIGINT (i64) as key field
\echo 'Test: BIGINT as key field'
CREATE TABLE bigint_test (
    id BIGINT,
    value TEXT
);

INSERT INTO bigint_test (id, value) VALUES (1, 'bluetooth');
INSERT INTO bigint_test (id, value) VALUES (2, 'bluebell');
INSERT INTO bigint_test (id, value) VALUES (3, 'jetblue');
INSERT INTO bigint_test (id, value) VALUES (4, 'blue''s clues');
INSERT INTO bigint_test (id, value) VALUES (5, 'blue bloods');
INSERT INTO bigint_test (id, value) VALUES (6, 'redness');
INSERT INTO bigint_test (id, value) VALUES (7, 'yellowtooth');
INSERT INTO bigint_test (id, value) VALUES (8, 'great white');
INSERT INTO bigint_test (id, value) VALUES (9, 'blue skies');
INSERT INTO bigint_test (id, value) VALUES (10, 'rainbow');

CREATE INDEX bigint_test_idx ON bigint_test USING bm25 (id, value)
WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');

-- Test stable sort (sorted by score)
\echo 'Query with ORDER BY score DESC for BIGINT key'
SELECT id, paradedb.score(id) FROM bigint_test WHERE bigint_test @@@ 
paradedb.term(field => 'value', value => 'blue') ORDER BY paradedb.score(id) DESC;

-- Test no stable sort
\echo 'Query without score sorting for BIGINT key'
SELECT id, paradedb.score(id) FROM bigint_test WHERE bigint_test @@@ 
paradedb.term(field => 'value', value => 'blue') ORDER BY id;

-- Test snippet function
\echo 'Testing paradedb.snippet with BIGINT key'
SELECT id, paradedb.snippet(value) FROM bigint_test WHERE value @@@ 'blue'
UNION
SELECT id, paradedb.snippet(value) FROM bigint_test WHERE value @@@ 'tooth'
ORDER BY id;

-- Test 2: UUID as key field
\echo 'Test: UUID as key field'
CREATE TABLE uuid_test (
    id UUID,
    value TEXT
);

INSERT INTO uuid_test (id, value) VALUES ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'bluetooth');
INSERT INTO uuid_test (id, value) VALUES ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'bluebell');
INSERT INTO uuid_test (id, value) VALUES ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'jetblue');
INSERT INTO uuid_test (id, value) VALUES ('eb833eb6-c598-4042-b84a-0045828fceea', 'blue''s clues');
INSERT INTO uuid_test (id, value) VALUES ('ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad', 'blue bloods');
INSERT INTO uuid_test (id, value) VALUES ('28b6374a-67d3-41c8-93af-490712f9923e', 'redness');
INSERT INTO uuid_test (id, value) VALUES ('f6e85626-298e-4112-9abb-3856f8aa046a', 'yellowtooth');
INSERT INTO uuid_test (id, value) VALUES ('88345d21-7b89-4fd6-87e4-83a4f68dbc3c', 'great white');
INSERT INTO uuid_test (id, value) VALUES ('40bc9216-66d0-4ae8-87ee-ddb02e3e1b33', 'blue skies');
INSERT INTO uuid_test (id, value) VALUES ('02f9789d-4963-47d5-a189-d9c114f5cba4', 'rainbow');

CREATE INDEX uuid_test_idx ON uuid_test USING bm25 (id, value)
WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');

-- Test stable sort (sorted by score)
\echo 'Query with ORDER BY score DESC for UUID key'
SELECT CAST(id AS TEXT), paradedb.score(id) FROM uuid_test WHERE uuid_test @@@ 
paradedb.term(field => 'value', value => 'blue') ORDER BY paradedb.score(id) DESC;

-- Test no stable sort
\echo 'Query without score sorting for UUID key'
SELECT CAST(id AS TEXT), paradedb.score(id) FROM uuid_test WHERE uuid_test @@@ 
paradedb.term(field => 'value', value => 'blue') ORDER BY id;

-- Test snippet function
\echo 'Testing paradedb.snippet with UUID key'
SELECT CAST(id AS TEXT), paradedb.snippet(value) FROM uuid_test WHERE value @@@ 'blue'
UNION
SELECT CAST(id AS TEXT), paradedb.snippet(value) FROM uuid_test WHERE value @@@ 'tooth'
ORDER BY id;

-- Test 3: TIMESTAMPTZ as key field
\echo 'Test: TIMESTAMP WITH TIME ZONE as key field'
CREATE TABLE timestamp_test (
    id TIMESTAMP WITH TIME ZONE,
    value TEXT
);

INSERT INTO timestamp_test (id, value) VALUES ('2023-05-03 08:09:10 EST', 'bluetooth');
INSERT INTO timestamp_test (id, value) VALUES ('2023-05-04 09:10:11 PST', 'bluebell');
INSERT INTO timestamp_test (id, value) VALUES ('2023-05-05 10:11:12 MST', 'jetblue');
INSERT INTO timestamp_test (id, value) VALUES ('2023-05-06 11:12:13 CST', 'blue''s clues');
INSERT INTO timestamp_test (id, value) VALUES ('2023-05-07 12:13:14 EST', 'blue bloods');
INSERT INTO timestamp_test (id, value) VALUES ('2023-05-08 13:14:15 PST', 'redness');
INSERT INTO timestamp_test (id, value) VALUES ('2023-05-09 14:15:16 MST', 'yellowtooth');
INSERT INTO timestamp_test (id, value) VALUES ('2023-05-10 15:16:17 CST', 'great white');
INSERT INTO timestamp_test (id, value) VALUES ('2023-05-11 16:17:18 EST', 'blue skies');
INSERT INTO timestamp_test (id, value) VALUES ('2023-05-12 17:18:19 PST', 'rainbow');

CREATE INDEX timestamp_test_idx ON timestamp_test USING bm25 (id, value)
WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');

-- Test stable sort (sorted by score)
\echo 'Query with ORDER BY score DESC for TIMESTAMPTZ key'
SELECT CAST(id AS TEXT), paradedb.score(id) FROM timestamp_test WHERE timestamp_test @@@ 
paradedb.term(field => 'value', value => 'blue') ORDER BY paradedb.score(id) DESC;

-- Test no stable sort
\echo 'Query without score sorting for TIMESTAMPTZ key'
SELECT CAST(id AS TEXT), paradedb.score(id) FROM timestamp_test WHERE timestamp_test @@@ 
paradedb.term(field => 'value', value => 'blue') ORDER BY id;

-- Test snippet function
\echo 'Testing paradedb.snippet with TIMESTAMPTZ key'
SELECT CAST(id AS TEXT), paradedb.snippet(value) FROM timestamp_test WHERE value @@@ 'blue'
UNION
SELECT CAST(id AS TEXT), paradedb.snippet(value) FROM timestamp_test WHERE value @@@ 'tooth'
ORDER BY id;

-- Clean up
DROP TABLE IF EXISTS bigint_test;
DROP TABLE IF EXISTS uuid_test;
DROP TABLE IF EXISTS timestamp_test; 
