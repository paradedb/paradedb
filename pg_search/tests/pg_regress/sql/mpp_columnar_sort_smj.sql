-- Distributed SortMergeJoin under MPP.
--
-- With `enable_columnar_sort`, an INNER join with both sides sorted on the join key lowers to a
-- `SortMergeJoinExec`. Under MPP that join dispatches as a broadcast merge join: the build (left)
-- side is broadcast to every worker task (`NetworkBroadcastExec`), re-sorted by the join key, while
-- the probe (right) side stays a work-stolen, sorted segment slice. Each task merges the full build
-- against its probe slice, so the per-segment sort is preserved (no re-sort) and matching keys are
-- co-located. The result must be byte-identical to serial, with no `running serially` fallback.

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

DROP TABLE IF EXISTS mpp_smj_posts CASCADE;
DROP TABLE IF EXISTS mpp_smj_users CASCADE;

CREATE TABLE mpp_smj_users (
    id INTEGER PRIMARY KEY,
    about_me TEXT COLLATE "C"
) WITH (autovacuum_enabled = false);

CREATE TABLE mpp_smj_posts (
    id INTEGER PRIMARY KEY,
    owner_user_id INTEGER,
    title TEXT COLLATE "C"
) WITH (autovacuum_enabled = false);

SET paradedb.global_mutable_segment_rows = 0;

-- Both sides sorted on their join-key column, so the join lowers to a `SortMergeJoinExec`.
CREATE INDEX mpp_smj_users_bm25 ON mpp_smj_users
USING bm25 (id, (about_me::pdb.unicode_words('columnar=true')))
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', target_segment_count = 8, background_layer_sizes = '0');

CREATE INDEX mpp_smj_posts_bm25 ON mpp_smj_posts
USING bm25 (id, owner_user_id, (title::pdb.unicode_words('columnar=true')))
WITH (key_field = 'id', sort_by = 'owner_user_id ASC NULLS FIRST', target_segment_count = 8, background_layer_sizes = '0');

-- Shuffled inserts so heap/ctid order differs from the sort-key order; a co-partitioning bug would
-- show as a lost row or a wrong order.
INSERT INTO mpp_smj_users (id, about_me)
SELECT i, 'about java code' FROM generate_series(1, 5000) i ORDER BY md5(i::text);

INSERT INTO mpp_smj_posts (id, owner_user_id, title)
SELECT i, (i % 5000) + 1, 'title ' || lpad(i::text, 6, '0')
FROM generate_series(1, 50000) i ORDER BY md5(i::text);

RESET paradedb.global_mutable_segment_rows;
ANALYZE;

-- Plan shape: a `DistributedExec` whose join is a `SortMergeJoinExec`, with the build side reaching
-- it through a `NetworkBroadcastExec` (full set per task) and the probe a single-partition sorted
-- `PgSearchScan` (segments=1). Stage/task counts are pinned by `mpp_worker_count`.
EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.owner_user_id, p.id
FROM mpp_smj_posts p
JOIN mpp_smj_users u ON p.owner_user_id = u.id
WHERE u.about_me ||| 'java' AND p.title ||| 'title'
ORDER BY p.owner_user_id ASC, p.id ASC
LIMIT 25;

-- Serial then MPP; the rows must match and the MPP run must not warn about a fallback.
SET paradedb.enable_mpp = off;
SELECT p.owner_user_id, p.id
FROM mpp_smj_posts p
JOIN mpp_smj_users u ON p.owner_user_id = u.id
WHERE u.about_me ||| 'java' AND p.title ||| 'title'
ORDER BY p.owner_user_id ASC, p.id ASC
LIMIT 25;

SET paradedb.enable_mpp = on;
SELECT p.owner_user_id, p.id
FROM mpp_smj_posts p
JOIN mpp_smj_users u ON p.owner_user_id = u.id
WHERE u.about_me ||| 'java' AND p.title ||| 'title'
ORDER BY p.owner_user_id ASC, p.id ASC
LIMIT 25;

-- Set-level equality gate over the full join (no LIMIT). The two rows must be identical apart from
-- the mode label.
SET paradedb.enable_mpp = off;
SELECT 'serial' AS mode, count(*) AS cnt, md5(string_agg(p.id::text, ',' ORDER BY p.id)) AS id_hash
FROM mpp_smj_posts p
JOIN mpp_smj_users u ON p.owner_user_id = u.id
WHERE u.about_me ||| 'java' AND p.title ||| 'title';

SET paradedb.enable_mpp = on;
SELECT 'mpp' AS mode, count(*) AS cnt, md5(string_agg(p.id::text, ',' ORDER BY p.id)) AS id_hash
FROM mpp_smj_posts p
JOIN mpp_smj_users u ON p.owner_user_id = u.id
WHERE u.about_me ||| 'java' AND p.title ||| 'title';

DROP TABLE mpp_smj_posts CASCADE;
DROP TABLE mpp_smj_users CASCADE;
