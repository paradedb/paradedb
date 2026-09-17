-- =====================================================================
-- A range-partitioned join over indexes built with `partition_by` on
-- existing rows. Each segment carries its partition's bounds in `.stats`, so
-- the join takes its split points from the build instead of sampling,
-- and each partition searches only the segments its range reaches.
-- Results must match the serial baseline.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_range_partitioned_join TO on;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET max_parallel_maintenance_workers TO 0;

CREATE FUNCTION sp_explain_analyze_lines(q text) RETURNS SETOF text AS $$
DECLARE r record;
BEGIN
  FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF) ' || q LOOP
    RETURN NEXT r."QUERY PLAN";
  END LOOP;
END $$ LANGUAGE plpgsql;

CREATE TABLE sp_users (id bigserial PRIMARY KEY, display_name text, about_me text);
CREATE TABLE sp_posts (id bigserial PRIMARY KEY, owner_user_id bigint, title text, body text);

INSERT INTO sp_users (display_name, about_me)
SELECT 'user_' || g, repeat('a', 900) || g FROM generate_series(1, 20000) g;
INSERT INTO sp_posts (owner_user_id, title, body)
SELECT 1 + ((g * 7919) % 20000), CASE WHEN g % 3 = 0 THEN 'error in build ' ELSE 'note ' END || g, repeat('b', 900) || g
FROM generate_series(1, 20000) g;

CREATE INDEX sp_users_idx ON sp_users USING paradedb (id, display_name)
WITH (partition_by = 'id', target_segment_count = 4,
      text_fields = '{"display_name": {"tokenizer": {"type": "keyword"}, "fast": true}}');
CREATE INDEX sp_posts_idx ON sp_posts USING paradedb (id, owner_user_id, title)
WITH (partition_by = 'owner_user_id', target_segment_count = 4,
      numeric_fields = '{"owner_user_id": {"fast": true}}');

SELECT relname, count(*) AS segments
FROM (SELECT 'sp_users_idx' AS relname FROM paradedb.index_info('sp_users_idx')
      UNION ALL SELECT 'sp_posts_idx' FROM paradedb.index_info('sp_posts_idx')) s
GROUP BY relname ORDER BY relname;

-- =====================================================================
-- Serial baseline.
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT u.id, p.id
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error'
ORDER BY u.id, p.id
LIMIT 10;

SELECT u.id, p.id
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error'
ORDER BY u.id, p.id
LIMIT 10;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*)
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error';

SELECT count(*)
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error';

-- =====================================================================
-- MPP: the scans show the build's boundaries, and the rows match.
-- =====================================================================

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT u.id, p.id
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error'
ORDER BY u.id, p.id
LIMIT 10;

SELECT u.id, p.id
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error'
ORDER BY u.id, p.id
LIMIT 10;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*)
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error';

SELECT count(*)
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error';

-- A range that reaches one of the four user segments: every task's scan of `u` reports one
-- candidate segment. Split points size this scan's partitions; the candidate count only sizes
-- them for scans without split points.
SELECT count(*) > 0 AND count(*) = count(*) FILTER (WHERE line ~ 'segments=1[,\]]')
       AS every_user_scan_reports_one_candidate_segment
FROM sp_explain_analyze_lines(
    $$SELECT count(*)
      FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
      WHERE u.id @@@ pdb.all() AND u.id BETWEEN 100 AND 200 AND p.title @@@ 'error'$$
) AS line
WHERE line ~ 'PgSearchScan: table=u,';

SELECT count(*)
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND u.id BETWEEN 100 AND 200 AND p.title @@@ 'error';

-- Result parity alone would pass if PgSearchScan always added the partition filter. This
-- metric counts predicates omitted at attachment because every selected segment is covered
-- and scores are not needed; it does not count per-segment scorer substitutions.
CREATE TEMP TABLE sp_range_filter_plan AS
SELECT line
FROM sp_explain_analyze_lines(
    $$SELECT count(*)
      FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
      WHERE u.id @@@ pdb.all() AND p.title @@@ 'error'$$
) AS line;

COPY (
    SELECT format(
        'range_filter_removed_in_execution=%s',
        EXISTS (
            SELECT 1 FROM sp_range_filter_plan
            WHERE line ~ 'range_filters_removed=([{][0-9]+:)?[1-9][0-9]*'
        )
    )
) TO STDOUT;

-- =====================================================================
-- One side's split points are enough. `sp_votes` is indexed empty and
-- filled afterwards, so its segments carry no box: the join cuts on the
-- users split points, and each votes partition is placed by its own range.
-- =====================================================================

CREATE TABLE sp_votes (id bigserial PRIMARY KEY, post_id bigint, kind text);
CREATE INDEX sp_votes_idx ON sp_votes USING paradedb (id, post_id, kind)
WITH (partition_by = 'post_id', target_segment_count = 4,
      numeric_fields = '{"post_id": {"fast": true}}',
      text_fields = '{"kind": {"tokenizer": {"type": "keyword"}, "fast": true}}');
INSERT INTO sp_votes (post_id, kind)
SELECT 1 + ((g * 31) % 10000), CASE WHEN g <= 10000 THEN 'up' ELSE 'down' END
FROM generate_series(1, 20000) g;
ANALYZE sp_votes;

SET max_parallel_workers_per_gather TO 0;

SELECT count(*)
FROM sp_users u JOIN sp_votes v ON u.id = v.post_id
WHERE u.id @@@ pdb.all() AND v.kind @@@ 'up';

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*)
FROM sp_users u JOIN sp_votes v ON u.id = v.post_id
WHERE u.id @@@ pdb.all() AND v.kind @@@ 'up';

SELECT count(*)
FROM sp_users u JOIN sp_votes v ON u.id = v.post_id
WHERE u.id @@@ pdb.all() AND v.kind @@@ 'up';

-- =====================================================================
-- No split points on either side: the join is not range partitioned.
-- =====================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*)
FROM sp_votes a JOIN sp_votes b ON a.post_id = b.post_id
WHERE a.kind @@@ 'up' AND b.kind @@@ 'down';

SELECT count(*)
FROM sp_votes a JOIN sp_votes b ON a.post_id = b.post_id
WHERE a.kind @@@ 'up' AND b.kind @@@ 'down';

-- =====================================================================
-- More workers than the split points seat: the plan caps its tasks, so
-- no task is empty and the join stays co-partitioned.
-- =====================================================================

SET max_parallel_workers_per_gather TO 6;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*)
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error';

SELECT count(*)
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error';

DROP TABLE sp_votes;

-- A cached plan may retain the exhaustive value grid, but not ownership by the segments visible
-- when it was planned. Late immutable segments must be mapped to the grid at execution and the
-- range predicate must return their matching row exactly once.
SET plan_cache_mode TO force_generic_plan;
PREPARE sp_cached_range_join AS
SELECT count(*)
FROM sp_users u JOIN sp_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error';

EXECUTE sp_cached_range_join;

SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO sp_users (id, display_name, about_me)
VALUES (25001, 'late_user', repeat('a', 900));
INSERT INTO sp_posts (id, owner_user_id, title, body)
VALUES (25001, 25001, 'error after cached plan', repeat('b', 900));
RESET paradedb.global_mutable_segment_rows;

SELECT relname, count(*) AS segments
FROM (SELECT 'sp_users_idx' AS relname FROM paradedb.index_info('sp_users_idx')
      UNION ALL SELECT 'sp_posts_idx' FROM paradedb.index_info('sp_posts_idx')) s
GROUP BY relname ORDER BY relname;

EXECUTE sp_cached_range_join;
DEALLOCATE sp_cached_range_join;
RESET plan_cache_mode;

DROP FUNCTION sp_explain_analyze_lines(text);
DROP TABLE sp_posts;
DROP TABLE sp_users;
