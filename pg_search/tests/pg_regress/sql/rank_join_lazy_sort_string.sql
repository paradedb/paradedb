-- Tests string and bytes (`NumericBytes`) columns as the primary ORDER BY key of the lazy-sorted probe
-- `HashJoinExec` rewrite in `RankJoinRule` (https://github.com/paradedb/paradedb/issues/7244).
-- Each query is run with the join custom scan on (lazy-sorted probe) and off (ground truth).

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_rank_join = on;
SET paradedb.rank_join_min_rows = 0;

DROP TABLE IF EXISTS lss_posts CASCADE;
DROP TABLE IF EXISTS lss_users CASCADE;

CREATE TABLE lss_users (
    id INT PRIMARY KEY,
    bio TEXT NOT NULL
) WITH (autovacuum_enabled = false);

CREATE TABLE lss_posts (
    id INT PRIMARY KEY,
    user_id INT NOT NULL,
    title TEXT COLLATE "C",
    tag TEXT COLLATE "C",
    amount NUMERIC(40, 0)
) WITH (autovacuum_enabled = false);

CREATE INDEX lss_users_bm25 ON lss_users USING paradedb (id, bio);
CREATE INDEX lss_posts_bm25 ON lss_posts USING paradedb (
    id, user_id, (title::pdb.unicode_words('columnar=true')),
    (tag::pdb.unicode_words('columnar=true')), amount
);

-- Two inserts per table produce multiple segments, so per-segment term ordinals differ.
SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO lss_users
SELECT i, CASE WHEN i % 10 = 0 THEN 'rare reviewer' ELSE 'common reader' END
FROM generate_series(1, 50) i;
INSERT INTO lss_users
SELECT i, CASE WHEN i % 10 = 0 THEN 'rare reviewer' ELSE 'common reader' END
FROM generate_series(51, 100) i;

INSERT INTO lss_posts
SELECT i,
       (i % 100) + 1,
       CASE WHEN i % 97 = 0 THEN NULL ELSE md5(i::text) END,
       't' || (i % 7),
       CASE WHEN i % 19 = 0 THEN NULL ELSE ((i * 7919) % 2001 - 1000) * 10::numeric ^ 25 + i END
FROM generate_series(1, 500) i;
INSERT INTO lss_posts
SELECT i,
       (i % 100) + 1,
       CASE WHEN i % 97 = 0 THEN NULL ELSE md5(i::text) END,
       't' || (i % 7),
       CASE WHEN i % 19 = 0 THEN NULL ELSE ((i * 7919) % 2001 - 1000) * 10::numeric ^ 25 + i END
FROM generate_series(501, 1000) i;

RESET paradedb.global_mutable_segment_rows;
ANALYZE lss_users, lss_posts;

-- =============================================================================
-- TEST 1: TEXT primary sort key (ASC)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.title
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.title ASC, p.id ASC
LIMIT 5;

SELECT p.id, p.title
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.title ASC, p.id ASC
LIMIT 5;

SET paradedb.enable_join_custom_scan = off;
SELECT p.id, p.title
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.title ASC, p.id ASC
LIMIT 5;
RESET paradedb.enable_join_custom_scan;

-- =============================================================================
-- TEST 2: TEXT primary sort key (DESC, NULLs first)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.title
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

SELECT p.id, p.title
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

SET paradedb.enable_join_custom_scan = off;
SELECT p.id, p.title
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.title DESC, p.id ASC
LIMIT 5;
RESET paradedb.enable_join_custom_scan;

-- =============================================================================
-- TEST 3: Bytes primary sort key (`NUMERIC` wider than 18 digits is stored as `NumericBytes`)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.amount
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.amount ASC, p.id ASC
LIMIT 5;

SELECT p.id, p.amount
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.amount ASC, p.id ASC
LIMIT 5;

SET paradedb.enable_join_custom_scan = off;
SELECT p.id, p.amount
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.amount ASC, p.id ASC
LIMIT 5;
RESET paradedb.enable_join_custom_scan;

-- =============================================================================
-- TEST 4: Low-cardinality TEXT primary sort key, with equal keys from both segments
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.user_id, p.tag
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.tag ASC, p.user_id ASC, p.id ASC
LIMIT 5;

SELECT p.id, p.user_id, p.tag
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.tag ASC, p.user_id ASC, p.id ASC
LIMIT 5;

SET paradedb.enable_join_custom_scan = off;
SELECT p.id, p.user_id, p.tag
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.tag ASC, p.user_id ASC, p.id ASC
LIMIT 5;
RESET paradedb.enable_join_custom_scan;

-- =============================================================================
-- TEST 5: First lazy-sort window is all dead rows (deleted without VACUUM), so the
-- leaf must re-query
-- =============================================================================

SET paradedb.enable_join_custom_scan = off;
DELETE FROM lss_posts WHERE id IN (
    SELECT p.id
    FROM lss_posts p
    WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
    ORDER BY p.title ASC, p.id ASC
    LIMIT 30
);
RESET paradedb.enable_join_custom_scan;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT p.id, p.title
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.title ASC, p.id ASC
LIMIT 5;

SELECT p.id, p.title
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.title ASC, p.id ASC
LIMIT 5;

SET paradedb.enable_join_custom_scan = off;
SELECT p.id, p.title
FROM lss_posts p
WHERE p.user_id IN (SELECT u.id FROM lss_users u WHERE u.bio ||| 'rare')
ORDER BY p.title ASC, p.id ASC
LIMIT 5;
RESET paradedb.enable_join_custom_scan;

DROP TABLE lss_posts CASCADE;
DROP TABLE lss_users CASCADE;
