-- =====================================================================
-- A multi-field `partition_by` cuts its first field into global ranges and
-- each later field inside the ranges of the fields before it. A range
-- co-partitioned join on the first field therefore reaches whole segments
-- only (`partial=0`), while a join on a later field still crosses segment
-- boxes. A `field=N` count fixes how many ranges a field is cut into.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_range_partitioned_join TO on;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET max_parallel_maintenance_workers TO 0;

CREATE TABLE pbh_users (id bigserial PRIMARY KEY, display_name text);
CREATE TABLE pbh_posts (id bigserial PRIMARY KEY, owner_user_id bigint, title text);
CREATE TABLE pbh_comments (id bigserial PRIMARY KEY, post_id bigint, body text);

INSERT INTO pbh_users (display_name)
SELECT 'user_' || g FROM generate_series(1, 4000) g;
INSERT INTO pbh_posts (owner_user_id, title)
-- Owners drift upward with `id`, the way early users post early, so the
-- `owner_user_id` ranges differ from one `id` range to the next.
SELECT 1 + ((g * 7919) % (2000 + g / 8)), CASE WHEN g % 3 = 0 THEN 'error in build ' ELSE 'note ' END || g
FROM generate_series(1, 16000) g;
INSERT INTO pbh_comments (post_id, body)
SELECT 1 + ((g * 104729) % 16000), 'question ' || g
FROM generate_series(1, 8000) g;

ANALYZE pbh_users;
ANALYZE pbh_posts;
ANALYZE pbh_comments;

CREATE INDEX pbh_users_idx ON pbh_users USING paradedb (id, display_name)
WITH (partition_by = 'id', target_segment_count = 4);
CREATE INDEX pbh_comments_idx ON pbh_comments USING paradedb (id, post_id, body)
WITH (partition_by = 'post_id', target_segment_count = 4);

-- =====================================================================
-- Default layout: `id` is cut into four global ranges, `owner_user_id`
-- into two inside each of them.
-- =====================================================================

CREATE INDEX pbh_posts_idx ON pbh_posts USING paradedb (id, owner_user_id, title)
WITH (partition_by = 'id, owner_user_id', target_segment_count = 8);

SELECT count(*) AS segments FROM paradedb.index_info('pbh_posts_idx');

SET max_parallel_workers_per_gather TO 3;

-- A join on the first field: every posts segment is whole in its task. The
-- posts side is the larger scan, so the join takes its split points.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, c.id
FROM pbh_posts p JOIN pbh_comments c ON c.post_id = p.id
WHERE p.id @@@ pdb.all() AND c.body ||| 'question'
ORDER BY p.id, c.id
LIMIT 10;

SELECT p.id, c.id
FROM pbh_posts p JOIN pbh_comments c ON c.post_id = p.id
WHERE p.id @@@ pdb.all() AND c.body ||| 'question'
ORDER BY p.id, c.id
LIMIT 10;

-- A join on the second field: its ranges differ from one `id` range to the
-- next, so the tasks reach into partial segments.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT u.id, p.id
FROM pbh_users u JOIN pbh_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error'
ORDER BY u.id, p.id
LIMIT 10;

SELECT u.id, p.id
FROM pbh_users u JOIN pbh_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title ||| 'error'
ORDER BY u.id, p.id
LIMIT 10;

-- =====================================================================
-- `id=8` spends every segment on `id`: each task takes whole segments and
-- `owner_user_id` is never cut.
-- =====================================================================

DROP INDEX pbh_posts_idx;
CREATE INDEX pbh_posts_idx ON pbh_posts USING paradedb (id, owner_user_id, title)
WITH (partition_by = 'id=8, owner_user_id', target_segment_count = 8);

SELECT count(*) AS segments FROM paradedb.index_info('pbh_posts_idx');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, c.id
FROM pbh_posts p JOIN pbh_comments c ON c.post_id = p.id
WHERE p.id @@@ pdb.all() AND c.body ||| 'question'
ORDER BY p.id, c.id
LIMIT 10;

SELECT p.id, c.id
FROM pbh_posts p JOIN pbh_comments c ON c.post_id = p.id
WHERE p.id @@@ pdb.all() AND c.body ||| 'question'
ORDER BY p.id, c.id
LIMIT 10;

-- =====================================================================
-- `id=2, owner_user_id=4` favors the second field: a join on `id` can only
-- split into the two `id` ranges.
-- =====================================================================

DROP INDEX pbh_posts_idx;
CREATE INDEX pbh_posts_idx ON pbh_posts USING paradedb (id, owner_user_id, title)
WITH (partition_by = 'id=2, owner_user_id=4', target_segment_count = 8);

SELECT count(*) AS segments FROM paradedb.index_info('pbh_posts_idx');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, c.id
FROM pbh_posts p JOIN pbh_comments c ON c.post_id = p.id
WHERE p.id @@@ pdb.all() AND c.body ||| 'question'
ORDER BY p.id, c.id
LIMIT 10;

SELECT p.id, c.id
FROM pbh_posts p JOIN pbh_comments c ON c.post_id = p.id
WHERE p.id @@@ pdb.all() AND c.body ||| 'question'
ORDER BY p.id, c.id
LIMIT 10;

-- =====================================================================
-- Invalid counts.
-- =====================================================================

CREATE INDEX pbh_bad_idx ON pbh_posts USING paradedb (id, owner_user_id, title)
WITH (partition_by = 'id=0, owner_user_id');
CREATE INDEX pbh_bad_idx ON pbh_posts USING paradedb (id, owner_user_id, title)
WITH (partition_by = 'id=eight, owner_user_id');
CREATE INDEX pbh_bad_idx ON pbh_posts USING paradedb (id, owner_user_id, title)
WITH (partition_by = 'id=, owner_user_id');
CREATE INDEX pbh_bad_idx ON pbh_posts USING paradedb (id, owner_user_id, title)
WITH (partition_by = '=4, owner_user_id');
-- The counts multiply past the target.
CREATE INDEX pbh_bad_idx ON pbh_posts USING paradedb (id, owner_user_id, title)
WITH (partition_by = 'id=4, owner_user_id=4', target_segment_count = 8);

DROP TABLE pbh_users;
DROP TABLE pbh_posts;
DROP TABLE pbh_comments;
