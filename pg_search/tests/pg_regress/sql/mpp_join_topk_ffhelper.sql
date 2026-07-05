-- Regression test for the FFHelper picked by a dispatched `SegmentedTopKExec`.
--
-- A top-k over a join sorts by a text fast field of the outer table. The deferred
-- sort column's `ff_index` is relative to that table's index. When the fragment is
-- dispatched to an MPP worker, the worker rewires the top-k to a scan in its subtree.
-- Picking the first scan instead of the one matching the column's `indexrelid` lands
-- on the joined table's helper, whose shorter fast-field list makes `ff_index` overrun
-- and the worker panics ("index out of bounds"), surfacing on the leader as
-- "transport receiver detached before this channel's EOF; the producer went away".
--
-- The outer table needs more fast fields than the joined one so a misindex overruns.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_mpp = on;
-- Regress tables are tiny; disable the size gate so MPP engages.
SET paradedb.mpp_min_rows TO 0;
SET paradedb.mpp_worker_count = 4;
SET paradedb.min_rows_per_worker = 0;
SET max_parallel_workers = 4;
SET max_parallel_workers_per_gather = 4;
SET parallel_tuple_cost = 0;
SET parallel_setup_cost = 0;
SET min_parallel_table_scan_size = 0;
SET min_parallel_index_scan_size = 0;
SET parallel_leader_participation = off;

DROP TABLE IF EXISTS mpp_topk_posts CASCADE;
DROP TABLE IF EXISTS mpp_topk_users CASCADE;

-- `C` collation keeps the byte-ordered sort that the top-k path requires, regardless of
-- the cluster's `initdb` locale.
CREATE TABLE mpp_topk_users (
    id INTEGER PRIMARY KEY,
    about_me TEXT COLLATE "C",
    display_name TEXT COLLATE "C"
) WITH (autovacuum_enabled = false);

CREATE TABLE mpp_topk_posts (
    id INTEGER PRIMARY KEY,
    owner_user_id INTEGER,
    title TEXT COLLATE "C",
    body TEXT COLLATE "C",
    tags TEXT COLLATE "C"
) WITH (autovacuum_enabled = false);

SET paradedb.global_mutable_segment_rows = 0;

-- The joined table has two text fast fields.
CREATE INDEX mpp_topk_users_bm25 ON mpp_topk_users
USING bm25 (
    id,
    (about_me::pdb.unicode_words('columnar=true')),
    (display_name::pdb.unicode_words('columnar=true'))
) WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0');

-- The sorted table has three, so `title`'s `ff_index` overruns the joined table's helper.
CREATE INDEX mpp_topk_posts_bm25 ON mpp_topk_posts
USING bm25 (
    id,
    owner_user_id,
    (title::pdb.unicode_words('columnar=true')),
    (body::pdb.unicode_words('columnar=true')),
    (tags::pdb.unicode_words('columnar=true'))
) WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0');

INSERT INTO mpp_topk_users (id, about_me, display_name)
SELECT i, 'about java code', 'David John Alex'
FROM generate_series(1, 5000) i;

INSERT INTO mpp_topk_posts (id, owner_user_id, title, body, tags)
SELECT i, (i % 5000) + 1, 'title ' || lpad(i::text, 6, '0') || ' code', 'body code text', 'tag'
FROM generate_series(1, 50000) i;

RESET paradedb.global_mutable_segment_rows;
ANALYZE;

-- No `EXPLAIN`: the dispatched plan prints per-task partition and segment counts that vary by
-- machine. Unique titles keep the top-k order deterministic across workers and locales.
SELECT p.id, p.title
FROM mpp_topk_posts p
WHERE p.owner_user_id IN (
    SELECT id FROM mpp_topk_users
    WHERE about_me ||| 'java' AND display_name ||| 'David'
)
ORDER BY p.title ASC
LIMIT 25;

DROP TABLE mpp_topk_posts CASCADE;
DROP TABLE mpp_topk_users CASCADE;
