-- Tests that MixedFF is used for UUIDs in the key field or in other fields.

\i common/common_setup.sql

\echo 'Test: Mixed field types in the same query'

DROP TABLE IF EXISTS products CASCADE;
CREATE TABLE products
(
    uuid_key  UUID NOT NULL PRIMARY KEY,
    uuid  UUID NOT NULL,
    name  TEXT
);

INSERT into products
    (uuid_key, uuid, name)
VALUES
    (gen_random_uuid(), gen_random_uuid(), 'alice'),
    (gen_random_uuid(), gen_random_uuid(), 'bob'),
    (gen_random_uuid(), gen_random_uuid(), 'bob'),
    (gen_random_uuid(), gen_random_uuid(), 'cloe'),
    (gen_random_uuid(), gen_random_uuid(), 'sally');

CREATE INDEX idxproducts ON products USING bm25 (uuid_key, uuid, name)
WITH (
    key_field = 'uuid_key',
    text_fields = '{
        "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
        "name": { "tokenizer": { "type": "keyword" }, "fast": true }
    }'
);

-- Confirm that the UUID key_field is fast and gets MixedFF.
SELECT name FROM products WHERE name @@@ 'bob' ORDER BY uuid_key;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT name FROM products WHERE name @@@ 'bob' ORDER BY uuid_key;

-- And that non-key UUID fields do too.
SELECT name FROM products WHERE name @@@ 'bob' ORDER BY uuid;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT name FROM products WHERE name @@@ 'bob' ORDER BY uuid;

\i common/common_cleanup.sql
