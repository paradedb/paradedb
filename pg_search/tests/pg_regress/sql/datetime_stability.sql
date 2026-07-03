-- Behaviors that should produce identical output before and after the
-- i64-PG-micros datetime storage transition. Internal representation
-- changes (tantivy Date / unix-epoch nanos -> tantivy I64 / pg-epoch
-- micros) must not be visible through these query shapes.
--
-- Intentionally NOT covered here: raw `pdb.agg('{"min":...}')` JSONB
-- output, which legitimately changes shape (gains `key_as_string`).

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS events CASCADE;

CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    description TEXT,
    occurred_at TIMESTAMP,
    occurred_at_tz TIMESTAMPTZ
);

INSERT INTO events (description, occurred_at, occurred_at_tz) VALUES
    ('alpha event',   '2024-01-01 10:00:00', '2024-01-01 10:00:00+00'),
    ('bravo event',   '2024-01-02 11:00:00', '2024-01-02 11:00:00+00'),
    ('charlie event', '2024-01-03 12:00:00', '2024-01-03 12:00:00+00'),
    ('delta event',   '2024-01-04 13:00:00', '2024-01-04 13:00:00+00'),
    ('echo event',    '2024-01-05 14:00:00', '2024-01-05 14:00:00+00');

CREATE INDEX events_idx ON events
USING bm25 (id, description, occurred_at, occurred_at_tz)
WITH (
    key_field = 'id',
    text_fields = '{"description": {}}'
);

-- =====================================================================
-- 1. Datetime-as-Datum returns round-trip identically
-- =====================================================================

SELECT id, occurred_at, occurred_at_tz
FROM events
WHERE id @@@ pdb.all()
ORDER BY id;

-- =====================================================================
-- 2. Range filter on a datetime field returns the same row set
-- =====================================================================

SELECT id
FROM events
WHERE occurred_at @@@ '[2024-01-02T00:00:00Z TO 2024-01-04T00:00:00Z}'
ORDER BY id;

SELECT id
FROM events
WHERE occurred_at_tz @@@ '[2024-01-02T00:00:00Z TO 2024-01-04T00:00:00Z}'
ORDER BY id;

-- =====================================================================
-- 3. TopK ordering on a datetime field
-- =====================================================================

SELECT id, occurred_at
FROM events
WHERE id @@@ pdb.all()
ORDER BY occurred_at
LIMIT 3;

SELECT id, occurred_at
FROM events
WHERE id @@@ pdb.all()
ORDER BY occurred_at DESC
LIMIT 3;

-- =====================================================================
-- 4. SQL-standard aggregates on datetime
-- =====================================================================

SELECT MIN(occurred_at), MAX(occurred_at), COUNT(*)
FROM events
WHERE id @@@ pdb.all();

SELECT MIN(occurred_at_tz), MAX(occurred_at_tz)
FROM events
WHERE id @@@ pdb.all();

-- =====================================================================
-- 5. GROUP BY on a datetime field
-- =====================================================================

SELECT occurred_at, COUNT(*)
FROM events
WHERE id @@@ pdb.all()
GROUP BY occurred_at
ORDER BY occurred_at;

-- =====================================================================
-- 6. date_histogram via paradedb.aggregate() — bucket keys and
--    key_as_string format must remain stable
-- =====================================================================

SELECT * FROM paradedb.aggregate(
    index => 'events_idx',
    query => paradedb.all(),
    agg   => '{"by_day": {"date_histogram": {"field": "occurred_at", "fixed_interval": "1d"}}}'
);

-- =====================================================================
-- 7. terms aggregation over a datetime field
-- =====================================================================

SELECT * FROM paradedb.aggregate(
    index => 'events_idx',
    query => paradedb.all(),
    agg   => '{"by_dt": {"terms": {"field": "occurred_at"}}}'
);

-- =====================================================================
-- 8. Plan shape: TopK on a datetime field still gets the fast-field plan
-- =====================================================================

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, occurred_at
FROM events
WHERE id @@@ pdb.all()
ORDER BY occurred_at
LIMIT 3;

-- =====================================================================
-- 9. date_histogram with a nested sub-agg — sub-bucket structure preserved
-- =====================================================================

SELECT * FROM paradedb.aggregate(
    index => 'events_idx',
    query => paradedb.all(),
    agg   => '{"by_day": {"date_histogram": {"field": "occurred_at", "fixed_interval": "1d"}, "aggs": {"count": {"value_count": {"field": "id"}}}}}'
);

-- Cleanup
DROP TABLE events CASCADE;
