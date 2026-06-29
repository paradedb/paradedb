-- Dispatch of sorted scans under MPP.
--
-- With `enable_columnar_sort`, a join source picks up a `sort_order` (the index `sort_by`, or the
-- `ctid` default), which used to lower to a multi-partition scan the MPP dispatch codec declined,
-- dropping the query to serial. The sorted source now ships as a single-partition leaf that claims
-- its segments and merges them inside `execute`, so these queries keep MPP parallelism. Each case
-- runs serial then MPP; the two must return byte-identical rows, and the MPP run must not warn
-- about a serial fallback.
--
-- A `SortMergeJoinExec` is still held back to serial (its distribution isn't correct yet); that is
-- covered by `mpp_columnar_sort_serial_fallback`.

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

DROP TABLE IF EXISTS mpp_cs_posts CASCADE;
DROP TABLE IF EXISTS mpp_cs_sorted CASCADE;
DROP TABLE IF EXISTS mpp_cs_users CASCADE;

CREATE TABLE mpp_cs_users (
    id INTEGER PRIMARY KEY,
    about_me TEXT COLLATE "C",
    display_name TEXT COLLATE "C"
) WITH (autovacuum_enabled = false);

-- Scenario A table: no `sort_by`, so the `ctid` default applies. The sort field is not projected,
-- so the leaf carries no declared ordering and chains its claimed segments (the no-merge path).
CREATE TABLE mpp_cs_posts (
    id INTEGER PRIMARY KEY,
    owner_user_id INTEGER,
    title TEXT COLLATE "C"
) WITH (autovacuum_enabled = false);

-- Scenario B table: sorted by `score_val`, a projected fast field that is not the join key, so the
-- semi-join stays a broadcast `HashJoin` and the leaf's per-segment merge runs in a dispatched plan.
CREATE TABLE mpp_cs_sorted (
    id INTEGER PRIMARY KEY,
    owner_user_id INTEGER,
    score_val INTEGER,
    title TEXT COLLATE "C"
) WITH (autovacuum_enabled = false);

SET paradedb.global_mutable_segment_rows = 0;

CREATE INDEX mpp_cs_users_bm25 ON mpp_cs_users
USING bm25 (
    id,
    (about_me::pdb.unicode_words('columnar=true')),
    (display_name::pdb.unicode_words('columnar=true'))
) WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0');

CREATE INDEX mpp_cs_posts_bm25 ON mpp_cs_posts
USING bm25 (
    id,
    owner_user_id,
    (title::pdb.unicode_words('columnar=true'))
) WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0');

CREATE INDEX mpp_cs_sorted_bm25 ON mpp_cs_sorted
USING bm25 (
    id,
    owner_user_id,
    score_val,
    (title::pdb.unicode_words('columnar=true'))
) WITH (key_field = 'id', sort_by = 'score_val ASC NULLS FIRST', target_segment_count = 8, background_layer_sizes = '0');

-- Insert shuffled so heap/ctid order differs from `score_val` order; `score_val = i*7919 % 100000`
-- is a bijection over these ids (no ties), so a merge bug shows as a wrong order or a lost row.
INSERT INTO mpp_cs_users (id, about_me, display_name)
SELECT i, 'about java code', 'David John Alex'
FROM generate_series(1, 5000) i ORDER BY md5(i::text);

INSERT INTO mpp_cs_posts (id, owner_user_id, title)
SELECT i, (i % 5000) + 1, 'title ' || lpad(i::text, 6, '0')
FROM generate_series(1, 50000) i ORDER BY md5(i::text);

INSERT INTO mpp_cs_sorted (id, owner_user_id, score_val, title)
SELECT i, (i % 5000) + 1, (i * 7919) % 100000, 'title ' || lpad(i::text, 6, '0')
FROM generate_series(1, 50000) i ORDER BY md5(i::text);

RESET paradedb.global_mutable_segment_rows;
ANALYZE;

-- Scenario A: `ctid`-default sort, the no-merge chain path on the partitioning source. Order is
-- driven by `ORDER BY title`. Serial first, then MPP; rows must match and the MPP run must not warn.
SET paradedb.enable_mpp = off;
SELECT p.id, p.title
FROM mpp_cs_posts p
WHERE p.owner_user_id IN (
    SELECT id FROM mpp_cs_users WHERE about_me ||| 'java' AND display_name ||| 'David'
)
ORDER BY p.title ASC
LIMIT 25;

SET paradedb.enable_mpp = on;
SELECT p.id, p.title
FROM mpp_cs_posts p
WHERE p.owner_user_id IN (
    SELECT id FROM mpp_cs_users WHERE about_me ||| 'java' AND display_name ||| 'David'
)
ORDER BY p.title ASC
LIMIT 25;

-- Scenario B: merge path. `score_val` is the index sort field and is projected, so the leaf
-- declares the ordering and merges its segments. `ORDER BY score_val` exercises the cross-worker
-- merge directly over a non-monotonic field.
SET paradedb.enable_mpp = off;
SELECT p.id, p.score_val
FROM mpp_cs_sorted p
WHERE p.owner_user_id IN (SELECT id FROM mpp_cs_users WHERE about_me ||| 'java')
ORDER BY p.score_val ASC, p.id ASC
LIMIT 25;

SET paradedb.enable_mpp = on;
SELECT p.id, p.score_val
FROM mpp_cs_sorted p
WHERE p.owner_user_id IN (SELECT id FROM mpp_cs_users WHERE about_me ||| 'java')
ORDER BY p.score_val ASC, p.id ASC
LIMIT 25;

-- Set-level equality gate over the full match set (no LIMIT) for both scenarios. Serial and MPP
-- must agree on count and id set; each pair of rows must be identical apart from the mode label.
SET paradedb.enable_mpp = off;
SELECT 'A serial' AS mode, count(*) AS cnt, md5(string_agg(id::text, ',' ORDER BY id)) AS id_hash
FROM mpp_cs_posts p
WHERE p.owner_user_id IN (SELECT id FROM mpp_cs_users WHERE about_me ||| 'java' AND display_name ||| 'David');
SET paradedb.enable_mpp = on;
SELECT 'A mpp' AS mode, count(*) AS cnt, md5(string_agg(id::text, ',' ORDER BY id)) AS id_hash
FROM mpp_cs_posts p
WHERE p.owner_user_id IN (SELECT id FROM mpp_cs_users WHERE about_me ||| 'java' AND display_name ||| 'David');

SET paradedb.enable_mpp = off;
SELECT 'B serial' AS mode, count(*) AS cnt, md5(string_agg(id::text, ',' ORDER BY id)) AS id_hash
FROM mpp_cs_sorted p
WHERE p.owner_user_id IN (SELECT id FROM mpp_cs_users WHERE about_me ||| 'java');
SET paradedb.enable_mpp = on;
SELECT 'B mpp' AS mode, count(*) AS cnt, md5(string_agg(id::text, ',' ORDER BY id)) AS id_hash
FROM mpp_cs_sorted p
WHERE p.owner_user_id IN (SELECT id FROM mpp_cs_users WHERE about_me ||| 'java');

-- Plan shape. Both scenarios dispatch to a `DistributedExec` whose inner join is a broadcast
-- `HashJoinExec` (not a `SortMergeJoinExec`), which is why they keep MPP parallelism rather than
-- fall back. The sorted leaf ships single-partition (`PgSearchScan: segments=1`); its per-segment
-- merge happens inside `execute`. The stage/task counts are pinned by `mpp_worker_count`.
SET paradedb.enable_mpp = on;
EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.title
FROM mpp_cs_posts p
WHERE p.owner_user_id IN (
    SELECT id FROM mpp_cs_users WHERE about_me ||| 'java' AND display_name ||| 'David'
)
ORDER BY p.title ASC
LIMIT 25;

EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.score_val
FROM mpp_cs_sorted p
WHERE p.owner_user_id IN (SELECT id FROM mpp_cs_users WHERE about_me ||| 'java')
ORDER BY p.score_val ASC, p.id ASC
LIMIT 25;

DROP TABLE mpp_cs_posts CASCADE;
DROP TABLE mpp_cs_sorted CASCADE;
DROP TABLE mpp_cs_users CASCADE;
