-- The `datetime_fields` option was deprecated in v0.24.1: datetimes are now
-- stored as i64 and the option has no effect. DDL from before v0.24.1 that
-- still contains `datetime_fields` must replay cleanly — a warning, not an
-- error (issue #5824).

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS deprecated_dt CASCADE;

CREATE TABLE deprecated_dt (
    id SERIAL8 NOT NULL PRIMARY KEY,
    ts TIMESTAMP,
    tstz TIMESTAMPTZ
);

INSERT INTO deprecated_dt (ts, tstz) VALUES
    ('2024-01-01 10:00:00', '2024-01-01 10:00:00+00'),
    ('2024-01-02 11:00:00', '2024-01-02 11:00:00+00'),
    ('2024-01-03 12:00:00', '2024-01-03 12:00:00+00');

-- Warns that the option is deprecated, but must not fail
CREATE INDEX deprecated_dt_idx ON deprecated_dt
USING paradedb (id, ts, tstz)
WITH (
    key_field = 'id',
    datetime_fields = '{"ts": {"fast": true}, "tstz": {"fast": true}}'
);

-- The index must behave as if the option had not been specified
SELECT id, ts, tstz
FROM deprecated_dt
WHERE id @@@ pdb.all()
ORDER BY id;

SELECT id
FROM deprecated_dt
WHERE ts @@@ '[2024-01-02T00:00:00Z TO 2024-01-03T00:00:00Z]'
ORDER BY id;

SELECT id, tstz
FROM deprecated_dt
WHERE id @@@ pdb.all()
ORDER BY tstz DESC
LIMIT 2;

-- Cleanup
DROP TABLE deprecated_dt CASCADE;
