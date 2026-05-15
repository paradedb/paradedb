-- TermSet queries against JSON-column datetime subpaths, exercising both
-- supported syntaxes:
--   * New API:    paradedb.term_set(terms => ARRAY[paradedb.term(...), ...])
--   * Legacy API: raw JSONB '{"term_set": {"terms": [{"field": ..., "value": ..., "is_datetime": true}, ...]}}'::jsonb
-- This locks in the OwnedValue::Date round-trip through CBOR for term_set
-- (which the existing term_set regression tests don't cover because they use
-- bigints/text), and the legacy is_datetime translation path for term_set.

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
USING bm25 (id, metadata)
WITH (
    key_field = 'id',
    json_fields = '{"metadata": {"fast": true}}'
);

-- ============================================================================
-- Datetime subpath: new API (paradedb.term_set + paradedb.term)
-- ============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM json_term_set_test
WHERE id @@@ paradedb.term_set(terms => ARRAY[
    paradedb.term('metadata.attributes.tstz', '2023-05-01T09:12:34Z'::timestamptz),
    paradedb.term('metadata.attributes.tstz', '2023-05-01T10:12:34Z'::timestamptz)
])
ORDER BY id;

SELECT id FROM json_term_set_test
WHERE id @@@ paradedb.term_set(terms => ARRAY[
    paradedb.term('metadata.attributes.tstz', '2023-05-01T09:12:34Z'::timestamptz),
    paradedb.term('metadata.attributes.tstz', '2023-05-01T10:12:34Z'::timestamptz)
])
ORDER BY id;

-- ============================================================================
-- Datetime subpath: legacy JSONB API with is_datetime
-- ============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM json_term_set_test
WHERE id @@@ '{
    "term_set": {
        "terms": [
            {"field": "metadata.attributes.tstz", "value": "2023-05-01T09:12:34Z", "is_datetime": true},
            {"field": "metadata.attributes.tstz", "value": "2023-05-01T10:12:34Z", "is_datetime": true}
        ]
    }
}'::jsonb
ORDER BY id;

SELECT id FROM json_term_set_test
WHERE id @@@ '{
    "term_set": {
        "terms": [
            {"field": "metadata.attributes.tstz", "value": "2023-05-01T09:12:34Z", "is_datetime": true},
            {"field": "metadata.attributes.tstz", "value": "2023-05-01T10:12:34Z", "is_datetime": true}
        ]
    }
}'::jsonb
ORDER BY id;

DROP TABLE json_term_set_test;
