-- =====================================================================
-- GROUP BY DATE(<timestamp>) pushdown
-- =====================================================================
-- `GROUP BY DATE(ts)` on a `timestamp` fast field is executed by the
-- DataFusion backend of the ParadeDB Aggregate Scan. DataFusion applies the
-- conversion with exact integer arithmetic and groups NULL values natively.
-- Timezone-sensitive and non-bare timestamp expressions still decline with
-- a named reason and fall back to Postgres.
--
-- Data notes:
--   * 2024-01-01 has three rows, including both edges of the day
--     (00:00:00 and 23:59:59); they must collapse into a single bucket.
--   * 2024-01-04 has no rows and must not appear as an empty bucket.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS events_nn CASCADE;
CREATE TABLE events_nn (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL
);

INSERT INTO events_nn (created_at) VALUES
    ('2024-01-01 00:00:00'),
    ('2024-01-01 08:00:00'),
    ('2024-01-01 23:59:59'),
    ('2024-01-02 09:00:00'),
    ('2024-01-03 12:00:00'),
    ('2024-01-05 18:00:00');

CREATE INDEX events_nn_idx ON events_nn USING paradedb (id, created_at)
WITH (key_field = 'id');

-- =====================================================================
-- Test 1: basic GROUP BY DATE(timestamp)
-- =====================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DATE(created_at), COUNT(*)
FROM events_nn
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at);

SELECT DATE(created_at) AS day, COUNT(*) AS cnt
FROM events_nn
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at)
ORDER BY day;

-- =====================================================================
-- Test 2: ts::date spelling resolves to the same function and pushes down
-- =====================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT created_at::date, COUNT(*)
FROM events_nn
WHERE id @@@ pdb.all()
GROUP BY created_at::date;

SELECT created_at::date AS day, COUNT(*) AS cnt
FROM events_nn
WHERE id @@@ pdb.all()
GROUP BY created_at::date
ORDER BY day;

DROP TABLE events_nn CASCADE;

-- =====================================================================
-- Test 3: nullable column — NULL group carries COUNT and SUM
-- =====================================================================
-- Arrow NULLs remain NULL after the timestamp-to-date UDF, so DataFusion
-- forms one NULL group carrying the same aggregate values as Postgres.

DROP TABLE IF EXISTS events_nullable CASCADE;
CREATE TABLE events_nullable (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP,
    region TEXT,
    amount INTEGER NOT NULL
);

INSERT INTO events_nullable (created_at, region, amount) VALUES
    ('2024-01-01 08:00:00', 'east', 10),
    ('2024-01-01 20:00:00', 'west', 20),
    ('2024-01-02 09:00:00', 'east', 40),
    (NULL, 'east', 100),
    (NULL, 'west', 200);

CREATE INDEX events_nullable_idx ON events_nullable
USING paradedb (id, created_at, region, amount)
WITH (key_field = 'id', text_fields = '{"region": {"fast": true}}');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DATE(created_at), COUNT(*), SUM(amount)
FROM events_nullable
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at);

SELECT DATE(created_at) AS day, COUNT(*) AS cnt, SUM(amount) AS total
FROM events_nullable
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at)
ORDER BY day NULLS LAST;

-- =====================================================================
-- Test 4: aggregate FILTER executes through DataFusion
-- =====================================================================
-- The filter is evaluated independently inside each date group. In
-- particular, the NULL group's filtered count must be 2, not 0.

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DATE(created_at), COUNT(*) FILTER (WHERE amount > 50)
FROM events_nullable
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at);

SELECT DATE(created_at) AS day, COUNT(*) FILTER (WHERE amount > 50) AS cnt
FROM events_nullable
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at)
ORDER BY day NULLS LAST;

-- =====================================================================
-- Test 5: multi-column GROUP BY executes through DataFusion
-- =====================================================================
-- DATE(created_at) uses the timestamp-to-date transform while region remains
-- an identity grouping expression. NULL dates must still split by region.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DATE(created_at), region, COUNT(*)
FROM events_nullable
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at), region;

SELECT DATE(created_at) AS day, region, COUNT(*) AS cnt
FROM events_nullable
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at), region
ORDER BY day NULLS LAST, region;

DROP TABLE events_nullable CASCADE;

-- =====================================================================
-- Test 6: DATE(timestamptz) declines with a named TimeZone reason
-- =====================================================================
-- date(timestamptz) [pg_proc 1178] depends on the session TimeZone, which
-- the plan cannot see; only date(timestamp) [2029] is pushed down. Under
-- America/Los_Angeles these two UTC instants fall on DIFFERENT local days,
-- which a timezone-free timestamp-to-date UDF could not reproduce.

DROP TABLE IF EXISTS events_tz CASCADE;
CREATE TABLE events_tz (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL
);

INSERT INTO events_tz (created_at) VALUES
    ('2024-01-02 05:00:00+00'),
    ('2024-01-02 20:00:00+00');

CREATE INDEX events_tz_idx ON events_tz USING paradedb (id, created_at)
WITH (key_field = 'id');

SET TIME ZONE 'America/Los_Angeles';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DATE(created_at), COUNT(*)
FROM events_tz
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at);

SELECT DATE(created_at) AS day, COUNT(*) AS cnt
FROM events_tz
WHERE id @@@ pdb.all()
GROUP BY DATE(created_at)
ORDER BY day;

RESET TIME ZONE;
DROP TABLE events_tz CASCADE;
