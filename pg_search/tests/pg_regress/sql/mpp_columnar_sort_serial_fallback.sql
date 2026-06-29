-- Regression test for the serial fallback when a plan can't be dispatched.
--
-- With `enable_columnar_sort`, an `ORDER BY` over a join lowers to a multi-partition (or
-- non-lazy) scan that the MPP dispatch codec ships only as a single-partition lazy leaf. The
-- leader must detect that shape and run the query serially, not error. Before the fix the
-- dispatch-build failure was a hard error ("only the lazy single-partition recipe is supported").
--
-- The query still returns the correct rows; the only visible sign of the fallback is the warning.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_columnar_sort = on;
SET paradedb.enable_mpp = on;
SET paradedb.mpp_worker_count = 4;
SET paradedb.min_rows_per_worker = 0;
SET max_parallel_workers = 4;
SET max_parallel_workers_per_gather = 4;
SET parallel_tuple_cost = 0;
SET parallel_setup_cost = 0;
SET min_parallel_table_scan_size = 0;
SET min_parallel_index_scan_size = 0;
SET parallel_leader_participation = off;

DROP TABLE IF EXISTS mpp_fb_posts CASCADE;
DROP TABLE IF EXISTS mpp_fb_users CASCADE;

CREATE TABLE mpp_fb_users (
    id INTEGER PRIMARY KEY,
    about_me TEXT COLLATE "C",
    display_name TEXT COLLATE "C"
) WITH (autovacuum_enabled = false);

CREATE TABLE mpp_fb_posts (
    id INTEGER PRIMARY KEY,
    owner_user_id INTEGER,
    title TEXT COLLATE "C",
    body TEXT COLLATE "C"
) WITH (autovacuum_enabled = false);

SET paradedb.global_mutable_segment_rows = 0;

CREATE INDEX mpp_fb_users_bm25 ON mpp_fb_users
USING bm25 (
    id,
    (about_me::pdb.unicode_words('columnar=true')),
    (display_name::pdb.unicode_words('columnar=true'))
) WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0');

CREATE INDEX mpp_fb_posts_bm25 ON mpp_fb_posts
USING bm25 (
    id,
    owner_user_id,
    (title::pdb.unicode_words('columnar=true')),
    (body::pdb.unicode_words('columnar=true'))
) WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0');

INSERT INTO mpp_fb_users (id, about_me, display_name)
SELECT i, 'about java code', 'David John Alex'
FROM generate_series(1, 5000) i;

INSERT INTO mpp_fb_posts (id, owner_user_id, title, body)
SELECT i, (i % 5000) + 1, 'title ' || lpad(i::text, 6, '0'), 'body code'
FROM generate_series(1, 50000) i;

RESET paradedb.global_mutable_segment_rows;
ANALYZE;

-- Undispatchable under MPP: must fall back to serial and return the rows, not error.
SELECT p.id, p.title
FROM mpp_fb_posts p
WHERE p.owner_user_id IN (
    SELECT id FROM mpp_fb_users
    WHERE about_me ||| 'java' AND display_name ||| 'David'
)
ORDER BY p.title ASC
LIMIT 25;

DROP TABLE mpp_fb_posts CASCADE;
DROP TABLE mpp_fb_users CASCADE;
