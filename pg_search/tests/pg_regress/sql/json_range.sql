-- Range queries against numeric and datetime JSON subpaths.

CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE json_range_test (
    id SERIAL PRIMARY KEY,
    metadata JSONB
);

INSERT INTO json_range_test (metadata) VALUES
    ('{"attributes": {"score": 3, "tstz": "2023-05-01T08:12:34Z"}}'),
    ('{"attributes": {"score": 4, "tstz": "2023-05-01T09:12:34Z"}}'),
    ('{"attributes": {"score": 5, "tstz": "2023-05-01T10:12:34Z"}}');

CREATE INDEX json_range_test_idx ON json_range_test
USING paradedb (id, (metadata::pdb.unicode_words('columnar=true')));

-- ============================================================================
-- Numeric subpath: V2 syntax (pdb.range + int4range)
-- ============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM json_range_test
WHERE (metadata -> 'attributes' -> 'score') @@@ pdb.range(int4range(4, NULL, '[)'))
ORDER BY id;

SELECT id FROM json_range_test
WHERE (metadata -> 'attributes' -> 'score') @@@ pdb.range(int4range(4, NULL, '[)'))
ORDER BY id;

-- ============================================================================
-- Datetime subpath: V2 syntax (pdb.range + tstzrange)
-- ============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM json_range_test
WHERE (metadata -> 'attributes' -> 'tstz') @@@ pdb.range(tstzrange('2023-05-01T09:12:00Z', NULL, '[)'))
ORDER BY id;

SELECT id FROM json_range_test
WHERE (metadata -> 'attributes' -> 'tstz') @@@ pdb.range(tstzrange('2023-05-01T09:12:00Z', NULL, '[)'))
ORDER BY id;

DROP TABLE json_range_test;
