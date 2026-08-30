-- =====================================================================
-- A `partition_by` index built with CREATE INDEX CONCURRENTLY. The scan
-- of a concurrent build cannot route, so a pass after it rewrites the
-- segments into partitions, and a range-partitioned join cuts on the
-- split points it recorded. Results must match the serial baseline.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_range_partitioned_join TO on;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET max_parallel_maintenance_workers TO 0;

CREATE TABLE cic_users (id bigserial PRIMARY KEY, display_name text, about_me text);
CREATE TABLE cic_posts (id bigserial PRIMARY KEY, owner_user_id bigint, title text, body text);

INSERT INTO cic_users (display_name, about_me)
SELECT 'user_' || g, repeat('a', 900) || g FROM generate_series(1, 20000) g;
INSERT INTO cic_posts (owner_user_id, title, body)
SELECT 1 + ((g * 7919) % 20000), CASE WHEN g % 3 = 0 THEN 'error in build ' ELSE 'note ' END || g, repeat('b', 900) || g
FROM generate_series(1, 20000) g;

CREATE INDEX CONCURRENTLY cic_users_idx ON cic_users USING paradedb (id, display_name)
WITH (key_field = 'id', partition_by = 'id', target_segment_count = 4,
      text_fields = '{"display_name": {"tokenizer": {"type": "keyword"}, "fast": true}}');
CREATE INDEX CONCURRENTLY cic_posts_idx ON cic_posts USING paradedb (id, owner_user_id, title)
WITH (key_field = 'id', partition_by = 'owner_user_id', target_segment_count = 4,
      numeric_fields = '{"owner_user_id": {"fast": true}}');

SELECT relname, count(*) AS segments
FROM (SELECT 'cic_users_idx' AS relname FROM paradedb.index_info('cic_users_idx')
      UNION ALL SELECT 'cic_posts_idx' FROM paradedb.index_info('cic_posts_idx')) s
GROUP BY relname ORDER BY relname;

-- =====================================================================
-- Serial baseline.
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

SELECT count(*)
FROM cic_users u JOIN cic_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error';

-- =====================================================================
-- MPP: the scans show the boundaries the post-build pass recorded, and
-- the rows match.
-- =====================================================================

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*)
FROM cic_users u JOIN cic_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error';

SELECT count(*)
FROM cic_users u JOIN cic_posts p ON u.id = p.owner_user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'error';

DROP TABLE cic_posts;
DROP TABLE cic_users;
