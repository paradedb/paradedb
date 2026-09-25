-- =====================================================================
-- Rows whose partition key is NULL belong to range partition 0. A LEFT
-- JOIN preserves posts without an owner, so the range-partitioned scan
-- of `rpn_posts` must produce them: an inner join could never tell. The
-- orphan count references `u.id` so the planner cannot remove the join.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan TO on;
RESET paradedb.enable_range_partitioned_join;
SET paradedb.mpp_min_rows TO 0;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET max_parallel_maintenance_workers TO 0;

CREATE TABLE rpn_users (id bigserial PRIMARY KEY, display_name text, about_me text);
CREATE TABLE rpn_posts (id bigserial PRIMARY KEY, owner_user_id bigint, title text, body text);

INSERT INTO rpn_users (display_name, about_me)
SELECT 'user_' || g, repeat('a', 900) || g FROM generate_series(1, 20000) g;
INSERT INTO rpn_posts (owner_user_id, title, body)
SELECT 1 + ((g * 7919) % 20000), CASE WHEN g % 3 = 0 THEN 'error in build ' ELSE 'note ' END || g, repeat('b', 900) || g
FROM generate_series(1, 20000) g;

CREATE INDEX rpn_users_idx ON rpn_users USING paradedb (id, (display_name::pdb.literal))
WITH (partition_by = 'id', target_segment_count = 4);
CREATE INDEX rpn_posts_idx ON rpn_posts USING paradedb (id, owner_user_id, title)
WITH (partition_by = 'owner_user_id', target_segment_count = 4);

SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO rpn_posts (owner_user_id, title, body)
SELECT NULL, 'error without owner ' || g, repeat('b', 900) || g
FROM generate_series(1, 7) g;
RESET paradedb.global_mutable_segment_rows;

SET max_parallel_workers_per_gather TO 0;

SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM rpn_posts p LEFT JOIN rpn_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title @@@ 'error';

-- The range-partitioned Left join must produce the same rows.
SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM rpn_posts p LEFT JOIN rpn_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title @@@ 'error';

SELECT count(*) AS total, count(*) FILTER (WHERE u.id IS NULL) AS orphans
FROM rpn_posts p LEFT JOIN rpn_users u ON u.id = p.owner_user_id AND u.id @@@ pdb.all()
WHERE p.title @@@ 'error';

DROP TABLE rpn_posts;
DROP TABLE rpn_users;

RESET paradedb.enable_join_custom_scan;
RESET paradedb.enable_range_partitioned_join;
RESET paradedb.mpp_min_rows;
RESET max_parallel_workers;
RESET max_parallel_workers_per_gather;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET max_parallel_maintenance_workers;
