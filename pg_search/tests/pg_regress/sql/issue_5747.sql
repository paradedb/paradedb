-- Issue 5747: DataFusion partition mismatch error on joins with different segment counts
--
-- The trigger is a join between two BM25-indexed tables whose indexes have
-- DIFFERENT physical segment counts. Equal segment counts do not reproduce it.
-- It requires an MPP-eligible worker configuration.
--
-- To get the asymmetry deterministically without needing much data:
--   * pc_big   uses mutable_segment_rows='0', so every INSERT lands in its own
--              immutable segment -> two INSERT statements give 2 segments.
--   * pc_small omits it, so its rows accumulate in one mutable segment -> 1.
\pset pager off
\set ON_ERROR_STOP off
SET max_parallel_workers_per_gather = 4;
DROP TABLE IF EXISTS pc_big, pc_small CASCADE;
CREATE TABLE pc_big   (id bigint PRIMARY KEY, state text);
CREATE TABLE pc_small (id bigint PRIMARY KEY, series_id bigint, user_id text);
CREATE INDEX pc_big_idx ON pc_big USING paradedb (id, state)
  WITH (key_field = 'id', target_segment_count = '3', mutable_segment_rows = '0');
CREATE INDEX pc_small_idx ON pc_small USING paradedb (id, series_id, user_id)
  WITH (key_field = 'id', target_segment_count = '3');
-- Two separate statements, so pc_big ends up with two immutable segments.
INSERT INTO pc_big SELECT g, 'active' FROM generate_series(1, 50) g;
INSERT INTO pc_big SELECT g, 'merged' FROM generate_series(51, 80) g;
-- One statement, so pc_small keeps a single segment.
INSERT INTO pc_small SELECT g, g, 'u1' FROM generate_series(1, 80) g;
EXPLAIN
SELECT le.id
FROM pc_small le
JOIN pc_big sv ON sv.id = le.series_id
WHERE le.id @@@ paradedb.term('user_id', 'u1')
  AND sv.id @@@ paradedb.term('state', 'merged')
ORDER BY le.id
LIMIT 5;

SELECT le.id
FROM pc_small le
JOIN pc_big sv ON sv.id = le.series_id
WHERE le.id @@@ paradedb.term('user_id', 'u1')
  AND sv.id @@@ paradedb.term('state', 'merged')
ORDER BY le.id
LIMIT 5;
DROP TABLE pc_big, pc_small CASCADE;
