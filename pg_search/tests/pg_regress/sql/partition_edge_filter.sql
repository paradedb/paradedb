-- =====================================================================
-- A range co-partitioned join checks a partition's edges in the segments
-- that cross them (`partial`). The check must only select rows: a NULL key
-- belongs to the first partition only, every value lands in one partition,
-- and a row scores the same as in a serial scan. A scan that a Top-K score
-- threshold reaches checks the edges on the rows the query returns, so the
-- threshold keeps pruning; EXPLAIN ANALYZE reports those rows as
-- `partition_rows_pruned`.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_range_partitioned_join TO on;
SET paradedb.mpp_min_rows TO 0;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET max_parallel_maintenance_workers TO 0;

CREATE TABLE pef_users (id bigserial PRIMARY KEY, display_name text);
CREATE TABLE pef_posts (id bigserial PRIMARY KEY, owner_user_id bigint, owner_name text, title text);

INSERT INTO pef_users (display_name)
SELECT 'user_' || lpad(g::text, 4, '0') FROM generate_series(1, 4000) g;
-- Owners drift upward with `id`, so the `owner_user_id` boxes of the posts
-- segments overlap and a join on that key crosses them.
INSERT INTO pef_posts (owner_user_id, owner_name, title)
SELECT o, 'user_' || lpad(o::text, 4, '0'), CASE WHEN g % 3 = 0 THEN 'error in build ' ELSE 'note ' END || g
FROM generate_series(1, 16000) g, LATERAL (SELECT 1 + ((g * 7919) % (2000 + g / 8)) AS o) owner;

CREATE INDEX pef_users_idx ON pef_users USING paradedb (id, (display_name::pdb.literal))
WITH (partition_by = 'id', target_segment_count = 4);
CREATE INDEX pef_posts_idx ON pef_posts USING paradedb (id, owner_user_id, (owner_name::pdb.literal), title)
WITH (partition_by = 'id, owner_user_id', target_segment_count = 8);

-- One more segment whose keys span every partition and include NULLs, so it
-- crosses every edge and its NULL rows must reach the first partition only.
SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO pef_posts (owner_user_id, owner_name, title)
SELECT o, 'user_' || lpad(o::text, 4, '0'), 'error without owner ' || g
FROM generate_series(1, 400) g, LATERAL (SELECT CASE WHEN g % 4 = 0 THEN NULL ELSE 1 + ((g * 7919) % 4000) END AS o) owner;
RESET paradedb.global_mutable_segment_rows;

ANALYZE pef_users;
ANALYZE pef_posts;

CREATE FUNCTION pef_explain_analyze_lines(q text) RETURNS SETOF text AS $$
DECLARE r record;
BEGIN
  FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF) ' || q LOOP
    RETURN NEXT r."QUERY PLAN";
  END LOOP;
END $$ LANGUAGE plpgsql;

-- Serial baselines.
SET max_parallel_workers_per_gather TO 0;

SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM pef_posts p LEFT JOIN pef_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

SELECT count(*) AS owner_join_rows
FROM pef_users u JOIN pef_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error';

SELECT p.id, pdb.score(p.id) AS score
FROM pef_users u JOIN pef_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error without owner'
ORDER BY p.id DESC
LIMIT 6;

SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM pef_posts p LEFT JOIN pef_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

-- =====================================================================
-- The range-partitioned join reaches into crossing segments in every task
-- and must produce the same rows and the same scores.
-- =====================================================================

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM pef_posts p LEFT JOIN pef_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM pef_posts p LEFT JOIN pef_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) AS owner_join_rows
FROM pef_users u JOIN pef_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error';

SELECT count(*) AS owner_join_rows
FROM pef_users u JOIN pef_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error';

-- Scored, with a Top-K on another column: no score threshold reaches the scan,
-- so the edges stay inside the query, and they add nothing to the score.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, pdb.score(p.id) AS score
FROM pef_users u JOIN pef_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error without owner'
ORDER BY p.id DESC
LIMIT 6;

SELECT p.id, pdb.score(p.id) AS score
FROM pef_users u JOIN pef_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error without owner'
ORDER BY p.id DESC
LIMIT 6;

-- Scored Top-K: the edges are checked on the rows the query returns.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM pef_posts p LEFT JOIN pef_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM pef_posts p LEFT JOIN pef_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

-- Asserts presence only: how the rows split across tasks is not pinned.
SELECT count(*) > 0 AS top_k_scans_check_the_edges_on_rows
FROM pef_explain_analyze_lines($q$
    SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
    FROM pef_posts p LEFT JOIN pef_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
    WHERE p.title ||| 'error without owner'
    ORDER BY pdb.score(p.id) DESC, p.id
    LIMIT 8
$q$) AS line
WHERE line LIKE '%table=p,%' AND line ~ 'partition_rows_pruned=\{[^}]*:[1-9][0-9]*';

SELECT count(*) = 0 AS unscored_scans_keep_the_edges_in_the_query
FROM pef_explain_analyze_lines($q$
    SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
    FROM pef_posts p LEFT JOIN pef_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
    WHERE p.title ||| 'error'
$q$) AS line
WHERE line LIKE '%partition_rows_pruned%';

-- =====================================================================
-- A text key: the edges are compared as strings, through the segment's
-- term ordinals when they are checked on rows.
-- =====================================================================

DROP INDEX pef_users_idx;
DROP INDEX pef_posts_idx;
CREATE INDEX pef_users_idx ON pef_users USING paradedb (id, (display_name::pdb.literal))
WITH (partition_by = 'display_name', target_segment_count = 4);
CREATE INDEX pef_posts_idx ON pef_posts USING paradedb (id, owner_user_id, (owner_name::pdb.literal), title)
WITH (partition_by = 'id, owner_name', target_segment_count = 8);

SET max_parallel_workers_per_gather TO 0;

SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM pef_posts p LEFT JOIN pef_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM pef_posts p LEFT JOIN pef_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM pef_posts p LEFT JOIN pef_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM pef_posts p LEFT JOIN pef_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM pef_posts p LEFT JOIN pef_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM pef_posts p LEFT JOIN pef_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

SELECT count(*) > 0 AS top_k_scans_check_the_text_edges_on_rows
FROM pef_explain_analyze_lines($q$
    SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
    FROM pef_posts p LEFT JOIN pef_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
    WHERE p.title ||| 'error without owner'
    ORDER BY pdb.score(p.id) DESC, p.id
    LIMIT 8
$q$) AS line
WHERE line LIKE '%table=p,%' AND line ~ 'partition_rows_pruned=\{[^}]*:[1-9][0-9]*';

DROP FUNCTION pef_explain_analyze_lines(text);
DROP TABLE pef_posts;
DROP TABLE pef_users;

RESET paradedb.enable_join_custom_scan;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_range_partitioned_join;
RESET paradedb.mpp_min_rows;
RESET max_parallel_workers;
RESET max_parallel_workers_per_gather;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET max_parallel_maintenance_workers;
