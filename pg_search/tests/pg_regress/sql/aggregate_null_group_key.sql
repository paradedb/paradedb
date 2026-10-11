-- A GROUP BY key that holds the largest or the smallest value of its type, next
-- to real NULLs. AggregateScan keys the NULL group with a string, so no value can
-- join it, and puts the group where ORDER BY puts NULLs.

\i common/common_setup.sql

CREATE TABLE null_group_keys (
    id SERIAL PRIMARY KEY,
    i8 BIGINT,
    f8 FLOAT8,
    ts TIMESTAMP,
    flag BOOLEAN,
    n INT
);

INSERT INTO null_group_keys (i8, f8, ts, flag, n) VALUES
    (-9223372036854775808, -1.7976931348623157e308, '-infinity', true, 1),
    (NULL, NULL, NULL, NULL, 10),
    (3, 1.5, '2024-01-01', false, 100),
    (9223372036854775807, 1.7976931348623157e308, 'infinity', true, 1000),
    (NULL, 2.5, '2024-06-01', true, 10000);

CREATE INDEX null_group_keys_idx ON null_group_keys
USING paradedb (id, i8, f8, ts, flag, n);

SET paradedb.enable_aggregate_custom_scan TO on;

\echo 'Test 1: the extremes of bigint keep their own groups, apart from the NULL group'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i8, COUNT(*) FROM null_group_keys WHERE id @@@ paradedb.all() GROUP BY i8 ORDER BY i8;

SELECT i8, COUNT(*) FROM null_group_keys WHERE id @@@ paradedb.all() GROUP BY i8 ORDER BY i8;

\echo 'Test 2: the NULL group goes where each ORDER BY puts NULLs'
SELECT i8, COUNT(*) FROM null_group_keys WHERE id @@@ paradedb.all() GROUP BY i8 ORDER BY i8 NULLS FIRST;
SELECT i8, COUNT(*) FROM null_group_keys WHERE id @@@ paradedb.all() GROUP BY i8 ORDER BY i8 DESC;
SELECT i8, COUNT(*) FROM null_group_keys WHERE id @@@ paradedb.all() GROUP BY i8 ORDER BY i8 DESC NULLS LAST;

\echo 'Test 3: float8 and timestamp extremes'
SELECT f8, COUNT(*) FROM null_group_keys WHERE id @@@ paradedb.all() GROUP BY f8 ORDER BY f8;
SELECT ts, COUNT(*) FROM null_group_keys WHERE id @@@ paradedb.all() GROUP BY ts ORDER BY ts DESC;

\echo 'Test 4: a metric over the group of an extreme value'
SELECT i8, SUM(n) FROM null_group_keys WHERE id @@@ paradedb.all() GROUP BY i8 ORDER BY i8;

\echo 'Test 5: a boolean NULL group sorts last'
SELECT flag, COUNT(*) FROM null_group_keys WHERE id @@@ paradedb.all() GROUP BY flag ORDER BY flag;

\echo 'Test 6: the NULL group of an inner level, next to values of the same outer group'
SELECT flag, i8, COUNT(*) FROM null_group_keys WHERE id @@@ paradedb.all()
GROUP BY flag, i8 ORDER BY flag, i8;

\echo 'Test 7: an aggregate FILTER'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i8, COUNT(*) FILTER (WHERE n > 1) FROM null_group_keys WHERE id @@@ paradedb.all()
GROUP BY i8 ORDER BY i8;

SELECT i8, COUNT(*) FILTER (WHERE n > 1) FROM null_group_keys WHERE id @@@ paradedb.all()
GROUP BY i8 ORDER BY i8;

\echo 'Test 8: a segment where every key is NULL'
CREATE TABLE null_group_segments (id SERIAL PRIMARY KEY, k BIGINT);
CREATE INDEX null_group_segments_idx ON null_group_segments USING paradedb (id, k);
SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO null_group_segments (k) VALUES (9223372036854775807), (1), (NULL);
INSERT INTO null_group_segments (k) VALUES (NULL), (NULL);
RESET paradedb.global_mutable_segment_rows;

SELECT count(*) AS segments FROM paradedb.index_info('null_group_segments_idx');

SELECT k, COUNT(*) FROM null_group_segments WHERE id @@@ paradedb.all() GROUP BY k ORDER BY k;

\echo 'Test 9: a GROUP BY truncated at the bucket cap'
-- Stale statistics keep the query on Tantivy past the cap; see
-- aggregate_truncation_guard.sql.
CREATE TABLE null_group_trunc (id BIGINT, k BIGINT) WITH (autovacuum_enabled = false);
INSERT INTO null_group_trunc SELECT g, g % 2 FROM generate_series(1, 100) g;
CREATE INDEX null_group_trunc_idx ON null_group_trunc USING paradedb (id, k);
ANALYZE null_group_trunc;
INSERT INTO null_group_trunc SELECT g, g FROM generate_series(1000, 1199) g;
INSERT INTO null_group_trunc VALUES (5000, NULL);
SET paradedb.max_term_agg_buckets = 10;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT k, COUNT(*) FROM null_group_trunc WHERE id @@@ paradedb.all() GROUP BY k ORDER BY k LIMIT 10;

-- Tantivy ranks the NULL group first, and the level keeps one bucket more than
-- the cap for it, so a window as wide as the cap is still all values.
SELECT k, COUNT(*) FROM null_group_trunc WHERE id @@@ paradedb.all() GROUP BY k ORDER BY k LIMIT 10;

-- DESC puts the NULL group first, and Tantivy sorted it past the cap.
SELECT k, COUNT(*) FROM null_group_trunc WHERE id @@@ paradedb.all() GROUP BY k ORDER BY k DESC LIMIT 5;

-- NULLS FIRST in ASC is where Tantivy puts the group.
SELECT k, COUNT(*) FROM null_group_trunc WHERE id @@@ paradedb.all() GROUP BY k ORDER BY k NULLS FIRST LIMIT 5;

RESET paradedb.max_term_agg_buckets;
RESET paradedb.enable_aggregate_custom_scan;
DROP TABLE null_group_keys;
DROP TABLE null_group_segments;
DROP TABLE null_group_trunc;
