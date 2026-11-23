-- Term-set queries on JSON datetime subpaths preserve OwnedValue::Date through CBOR.

CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE json_term_set_test (
    id SERIAL PRIMARY KEY,
    metadata JSONB
);

INSERT INTO json_term_set_test (metadata) VALUES
    ('{"attributes": {"tstz": "2023-05-01T08:12:34Z"}}'),
    ('{"attributes": {"tstz": "2023-05-01T09:12:34Z"}}'),
    ('{"attributes": {"tstz": "2023-05-01T10:12:34Z"}}');

CREATE INDEX json_term_set_test_idx ON json_term_set_test
USING paradedb (id, (metadata::pdb.unicode_words('columnar=true')));

-- ============================================================================
-- Datetime subpath: typed V2 term set
-- ============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM json_term_set_test
WHERE (metadata -> 'attributes' -> 'tstz') @@@ pdb.term_set(ARRAY[
    '2023-05-01T09:12:34Z'::timestamptz,
    '2023-05-01T10:12:34Z'::timestamptz
])
ORDER BY id;

SELECT id FROM json_term_set_test
WHERE (metadata -> 'attributes' -> 'tstz') @@@ pdb.term_set(ARRAY[
    '2023-05-01T09:12:34Z'::timestamptz,
    '2023-05-01T10:12:34Z'::timestamptz
])
ORDER BY id;

DROP TABLE json_term_set_test;
