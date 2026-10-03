-- =====================================================================
-- A range co-partitioned join checks a partition's edges in the segments
-- that cross them (`partial`). The check must only select rows: a NULL key
-- belongs to the first partition only, every value lands in one partition,
-- and a row scores the same as in a serial scan, on a numeric key and on a
-- text key.
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

CREATE TABLE rpx_users (id bigserial PRIMARY KEY, display_name text);
CREATE TABLE rpx_posts (id bigserial PRIMARY KEY, owner_user_id bigint, owner_name text, title text);

INSERT INTO rpx_users (display_name)
SELECT 'user_' || lpad(g::text, 4, '0') FROM generate_series(1, 4000) g;
-- Owners drift upward with `id`, so the `owner_user_id` boxes of the posts
-- segments overlap and a join on that key crosses them.
INSERT INTO rpx_posts (owner_user_id, owner_name, title)
SELECT o, 'user_' || lpad(o::text, 4, '0'), CASE WHEN g % 3 = 0 THEN 'error in build ' ELSE 'note ' END || g
FROM generate_series(1, 16000) g, LATERAL (SELECT 1 + ((g * 7919) % (2000 + g / 8)) AS o) owner;

CREATE INDEX rpx_users_idx ON rpx_users USING paradedb (id, (display_name::pdb.literal))
WITH (partition_by = 'id', target_segment_count = 4);
CREATE INDEX rpx_posts_idx ON rpx_posts USING paradedb (id, owner_user_id, (owner_name::pdb.literal), title)
WITH (partition_by = 'id, owner_user_id', target_segment_count = 8);

-- One more segment whose keys span every partition and include NULLs, so it
-- crosses every edge and its NULL rows must reach the first partition only.
SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO rpx_posts (owner_user_id, owner_name, title)
SELECT o, 'user_' || lpad(o::text, 4, '0'), 'error without owner ' || g
FROM generate_series(1, 400) g, LATERAL (SELECT CASE WHEN g % 4 = 0 THEN NULL ELSE 1 + ((g * 7919) % 4000) END AS o) owner;
RESET paradedb.global_mutable_segment_rows;

ANALYZE rpx_users;
ANALYZE rpx_posts;

-- Serial baselines.
SET max_parallel_workers_per_gather TO 0;

SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM rpx_posts p LEFT JOIN rpx_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

SELECT count(*) AS owner_join_rows
FROM rpx_users u JOIN rpx_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error';

SELECT p.id, pdb.score(p.id) AS score
FROM rpx_users u JOIN rpx_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error without owner'
ORDER BY p.id DESC
LIMIT 6;

SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM rpx_posts p LEFT JOIN rpx_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
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
FROM rpx_posts p LEFT JOIN rpx_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM rpx_posts p LEFT JOIN rpx_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) AS owner_join_rows
FROM rpx_users u JOIN rpx_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error';

SELECT count(*) AS owner_join_rows
FROM rpx_users u JOIN rpx_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error';

-- Scored, with a Top K on another column.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, pdb.score(p.id) AS score
FROM rpx_users u JOIN rpx_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error without owner'
ORDER BY p.id DESC
LIMIT 6;

SELECT p.id, pdb.score(p.id) AS score
FROM rpx_users u JOIN rpx_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error without owner'
ORDER BY p.id DESC
LIMIT 6;

-- Top K by score. The rows without an owner are in the first partition only.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM rpx_posts p LEFT JOIN rpx_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM rpx_posts p LEFT JOIN rpx_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

-- =====================================================================
-- A text key.
-- =====================================================================

DROP INDEX rpx_users_idx;
DROP INDEX rpx_posts_idx;
CREATE INDEX rpx_users_idx ON rpx_users USING paradedb (id, (display_name::pdb.literal))
WITH (partition_by = 'display_name', target_segment_count = 4);
CREATE INDEX rpx_posts_idx ON rpx_posts USING paradedb (id, owner_user_id, (owner_name::pdb.literal), title)
WITH (partition_by = 'id, owner_name', target_segment_count = 8);

SET max_parallel_workers_per_gather TO 0;

SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM rpx_posts p LEFT JOIN rpx_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM rpx_posts p LEFT JOIN rpx_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM rpx_posts p LEFT JOIN rpx_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM rpx_posts p LEFT JOIN rpx_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM rpx_posts p LEFT JOIN rpx_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

SELECT p.id, pdb.score(p.id) AS score, u.id IS NULL AS orphan
FROM rpx_posts p LEFT JOIN rpx_users u ON u.display_name = p.owner_name AND u.id @@@ pdb.all()
WHERE p.title ||| 'error without owner'
ORDER BY pdb.score(p.id) DESC, p.id
LIMIT 8;

DROP TABLE rpx_posts;
DROP TABLE rpx_users;

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
