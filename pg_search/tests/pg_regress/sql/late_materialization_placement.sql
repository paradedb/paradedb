-- The placement rule decides per source where the two halves of a late-materialized
-- string lookup run. A build side comes back out of doc order, and a join key that is not
-- the other side's key field fans the rows out; either moves the fetch into the scan. A
-- fan-out with nothing above that stops after a fixed number of rows moves the decode into
-- the scan as well.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO off;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_aggregate_custom_scan = on;

DROP TABLE IF EXISTS lmp_comments CASCADE;
DROP TABLE IF EXISTS lmp_posts CASCADE;

CREATE TABLE lmp_posts (
    id SERIAL PRIMARY KEY,
    title TEXT,
    body TEXT
);

CREATE TABLE lmp_comments (
    id SERIAL PRIMARY KEY,
    post_id INTEGER,
    author TEXT,
    body TEXT
);

CREATE INDEX lmp_posts_idx ON lmp_posts USING bm25 (id, title, body)
WITH (key_field = 'id', text_fields = '{"title": {"fast": true}, "body": {}}');

CREATE INDEX lmp_comments_idx ON lmp_comments USING bm25 (id, post_id, author, body)
WITH (key_field = 'id', numeric_fields = '{"post_id": {"fast": true}}', text_fields = '{"author": {"fast": true}, "body": {}}');

-- Two insert batches per table give each index two segments, enough for the MPP leg to
-- cut the plan into tasks.
SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO lmp_posts (title, body)
SELECT
    'Post ' || LPAD(i::TEXT, 3, '0'),
    CASE WHEN i % 3 = 0 THEN 'alpha topic' ELSE 'beta topic' END
FROM generate_series(1, 15) AS i;

INSERT INTO lmp_posts (title, body)
SELECT
    'Post ' || LPAD(i::TEXT, 3, '0'),
    CASE WHEN i % 3 = 0 THEN 'alpha topic' ELSE 'beta topic' END
FROM generate_series(16, 30) AS i;

-- Five comments per post, so a join on `post_id` fans a post's row out.
INSERT INTO lmp_comments (post_id, author, body)
SELECT
    (i - 1) % 30 + 1,
    'author ' || LPAD(((i * 7) % 11)::TEXT, 2, '0'),
    'comment ' || i
FROM generate_series(1, 75) AS i;

INSERT INTO lmp_comments (post_id, author, body)
SELECT
    (i - 1) % 30 + 1,
    'author ' || LPAD(((i * 7) % 11)::TEXT, 2, '0'),
    'comment ' || i
FROM generate_series(76, 150) AS i;

RESET paradedb.global_mutable_segment_rows;

SHOW paradedb.defer_column_fetch;
SHOW paradedb.defer_string_decode;

-- =============================================================================
-- Top-K: the consumer is bounded, so the decode stays deferred either way
-- =============================================================================

-- The sort key is on the build side, and `post_id` is not the key field of the comments
-- index: the posts scan resolves the ordinals itself.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.title
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

SELECT p.id, p.title
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

-- The sort key is on the probe side, and `id` is the key field of the posts index: the
-- rows reach the decode point in doc order and at most once, so the fetch stays deferred.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.author
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY c.author ASC, c.id ASC
LIMIT 5;

SELECT c.id, c.author
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY c.author ASC, c.id ASC
LIMIT 5;

-- A non-equi join fans out whatever the keys are.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.author
FROM lmp_posts p JOIN lmp_comments c ON c.post_id < p.id
WHERE p.body @@@ 'alpha'
ORDER BY c.author ASC, c.id ASC
LIMIT 5;

SELECT c.id, c.author
FROM lmp_posts p JOIN lmp_comments c ON c.post_id < p.id
WHERE p.body @@@ 'alpha'
ORDER BY c.author ASC, c.id ASC
LIMIT 5;

-- Without the SegmentedTopKExec the sort consumes every row, so a fan-out decodes
-- in the scan like an aggregate would.
SET paradedb.enable_segmented_topk = off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.title
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

SELECT p.id, p.title
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

RESET paradedb.enable_segmented_topk;

-- A semi-join never multiplies the outer rows; the build side still fetches in the scan.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.title
FROM lmp_posts p
WHERE p.body @@@ 'alpha'
  AND EXISTS (SELECT 1 FROM lmp_comments c WHERE c.post_id = p.id)
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

SELECT p.id, p.title
FROM lmp_posts p
WHERE p.body @@@ 'alpha'
  AND EXISTS (SELECT 1 FROM lmp_comments c WHERE c.post_id = p.id)
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

-- An outer join keeps the same key shape as the inner one.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.title
FROM lmp_posts p LEFT JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

SELECT p.id, p.title
FROM lmp_posts p LEFT JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

-- Two scans of one index share one decision.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT a.id, a.author, b.author
FROM lmp_comments a JOIN lmp_comments b ON a.post_id = b.post_id
WHERE a.body @@@ 'comment'
ORDER BY a.author ASC, b.author DESC, a.id ASC, b.id ASC
LIMIT 5;

SELECT a.id, a.author, b.author
FROM lmp_comments a JOIN lmp_comments b ON a.post_id = b.post_id
WHERE a.body @@@ 'comment'
ORDER BY a.author ASC, b.author DESC, a.id ASC, b.id ASC
LIMIT 5;

-- In a three-table join, a scan on the build side of the inner join fetches in the
-- scan; the decode stays under the Top-K.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.author
FROM lmp_comments c
JOIN (lmp_posts p JOIN lmp_comments c2 ON c2.post_id = p.id) ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY c.author ASC, c.id ASC, c2.id ASC
LIMIT 5;

SELECT c.id, c.author
FROM lmp_comments c
JOIN (lmp_posts p JOIN lmp_comments c2 ON c2.post_id = p.id) ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY c.author ASC, c.id ASC, c2.id ASC
LIMIT 5;

-- =============================================================================
-- Aggregates: a key with few rows per term is not grouped on ordinals, so
-- nothing bounds the fan-out and the scan decodes it
-- =============================================================================

SET paradedb.enable_aggregate_late_materialization = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.title, COUNT(*)
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
GROUP BY p.title
ORDER BY p.title
LIMIT 5;

SELECT p.title, COUNT(*)
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
GROUP BY p.title
ORDER BY p.title
LIMIT 5;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.author, COUNT(*)
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
GROUP BY c.author
ORDER BY c.author
LIMIT 5;

SELECT c.author, COUNT(*)
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
GROUP BY c.author
ORDER BY c.author
LIMIT 5;

-- =============================================================================
-- Settings override the rule
-- =============================================================================

SET paradedb.defer_string_decode = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.title, COUNT(*)
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
GROUP BY p.title
ORDER BY p.title
LIMIT 5;

SELECT p.title, COUNT(*)
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
GROUP BY p.title
ORDER BY p.title
LIMIT 5;

RESET paradedb.defer_string_decode;

-- A pinned fetch stays with the decode; when the decode runs in the scan, so does it.
SET paradedb.defer_column_fetch = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.title, COUNT(*)
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
GROUP BY p.title
ORDER BY p.title
LIMIT 5;

SELECT p.title, COUNT(*)
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
GROUP BY p.title
ORDER BY p.title
LIMIT 5;

RESET paradedb.defer_column_fetch;
RESET paradedb.enable_aggregate_late_materialization;

SET paradedb.defer_column_fetch = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.title
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

SELECT p.id, p.title
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

RESET paradedb.defer_column_fetch;

-- With the decode never deferred, no union leaves the scan and the sort runs on strings.
SET paradedb.defer_string_decode = off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.title
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

SELECT p.id, p.title
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

RESET paradedb.defer_string_decode;

-- =============================================================================
-- MPP: the decision is made before the plan is cut into stages
-- =============================================================================

SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.title
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

SELECT p.id, p.title
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
ORDER BY p.title DESC, p.id ASC
LIMIT 5;

SET paradedb.enable_aggregate_late_materialization = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.title, COUNT(*)
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
GROUP BY p.title
ORDER BY p.title
LIMIT 5;

SELECT p.title, COUNT(*)
FROM lmp_posts p JOIN lmp_comments c ON c.post_id = p.id
WHERE p.body @@@ 'alpha'
GROUP BY p.title
ORDER BY p.title
LIMIT 5;

RESET paradedb.enable_aggregate_late_materialization;

DROP TABLE lmp_comments;
DROP TABLE lmp_posts;
