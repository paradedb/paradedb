-- =====================================================================
-- A range-partitioned join over indexes built with `partition_by` on
-- existing rows. Each segment carries its cell's bounds in `.stats`, so
-- the join takes its split points from the build instead of sampling,
-- and each partition searches only the segments its range reaches.
-- Results must match the serial baseline. The heaps exceed the 15MB
-- floor below which a build collapses to a single cell.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_range_partitioned_join TO on;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET max_parallel_maintenance_workers TO 0;

CREATE TABLE sp_users (id bigserial PRIMARY KEY, display_name text, about_me text);
CREATE TABLE sp_posts (id bigserial PRIMARY KEY, owner_user_id bigint, title text, body text);

INSERT INTO sp_users (display_name, about_me)
SELECT 'user_' || g, repeat('a', 900) || g FROM generate_series(1, 20000) g;
INSERT INTO sp_posts (owner_user_id, title, body)
SELECT 1 + ((g * 7919) % 20000), CASE WHEN g % 3 = 0 THEN 'error in build ' ELSE 'note ' END || g, repeat('b', 900) || g
FROM generate_series(1, 20000) g;

CREATE INDEX sp_users_idx ON sp_users USING paradedb (id, display_name)
WITH (key_field = 'id', partition_by = 'id', target_segment_count = 4,
      text_fields = '{"display_name": {"tokenizer": {"type": "keyword"}, "fast": true}}');
CREATE INDEX sp_posts_idx ON sp_posts USING paradedb (id, owner_user_id, title)
WITH (key_field = 'id', partition_by = 'owner_user_id', target_segment_count = 4,
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

DROP TABLE sp_posts;
DROP TABLE sp_users;
