-- A list column (here `text[]`) that a DataFusion plan reads from the index leaves its scan
-- as a doc address and is read and decoded once, at the point that consumes it, the way a
-- string column is. One row of a list holds many terms, so a list has no term ordinal step:
-- no fetch node takes it and `paradedb.defer_column_fetch` does not apply to it. Each shape
-- is checked again with `paradedb.defer_string_decode = off`, which reads every list in the
-- scan; the rows must match.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO off;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_aggregate_custom_scan = on;

DROP TABLE IF EXISTS dac_comments CASCADE;
DROP TABLE IF EXISTS dac_posts CASCADE;

CREATE TABLE dac_posts (
    id SERIAL PRIMARY KEY,
    title TEXT,
    tags TEXT[]
);

CREATE TABLE dac_comments (
    id SERIAL PRIMARY KEY,
    post_id INTEGER,
    author TEXT,
    labels TEXT[]
);

CREATE INDEX dac_posts_idx ON dac_posts USING paradedb (id, title, (tags::pdb.literal));

CREATE INDEX dac_comments_idx ON dac_comments USING paradedb (id, post_id, (author::pdb.literal), (labels::pdb.literal));

-- Two insert batches per table give each index two segments, so one batch above a join
-- holds rows of both. Some arrays are NULL and some are empty.
SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO dac_posts (title, tags)
SELECT
    CASE WHEN i % 3 = 0 THEN 'alpha post ' || i ELSE 'beta post ' || i END,
    CASE WHEN i % 7 = 0 THEN NULL WHEN i % 5 = 0 THEN ARRAY[]::TEXT[] ELSE ARRAY['t' || (i % 4), 'all'] END
FROM generate_series(1, 15) AS i;

INSERT INTO dac_posts (title, tags)
SELECT
    CASE WHEN i % 3 = 0 THEN 'alpha post ' || i ELSE 'beta post ' || i END,
    CASE WHEN i % 7 = 0 THEN NULL WHEN i % 5 = 0 THEN ARRAY[]::TEXT[] ELSE ARRAY['t' || (i % 4), 'all'] END
FROM generate_series(16, 30) AS i;

-- Three comments on each of posts 1 to 20; posts 21 to 30 have none.
INSERT INTO dac_comments (post_id, author, labels)
SELECT
    (i - 1) % 20 + 1,
    'author ' || (i % 4),
    CASE WHEN i % 6 = 0 THEN NULL WHEN i % 9 = 0 THEN ARRAY[]::TEXT[] ELSE ARRAY['l' || (i % 3), 'c' || (i % 2)] END
FROM generate_series(1, 30) AS i;

INSERT INTO dac_comments (post_id, author, labels)
SELECT
    (i - 1) % 20 + 1,
    'author ' || (i % 4),
    CASE WHEN i % 6 = 0 THEN NULL WHEN i % 9 = 0 THEN ARRAY[]::TEXT[] ELSE ARRAY['l' || (i % 3), 'c' || (i % 2)] END
FROM generate_series(31, 60) AS i;

RESET paradedb.global_mutable_segment_rows;

SHOW paradedb.defer_column_fetch;
SHOW paradedb.defer_string_decode;

-- =============================================================================
-- Aggregating over an unnested list
-- =============================================================================

-- `id` is the primary key of the posts, so a comment meets at most one post: the labels
-- cross the join as doc addresses and are read once, just below the unnest.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY l
ORDER BY l;

SELECT l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY l
ORDER BY l;

-- A post meets each of its comments, so its tags would be read once per comment above the
-- join. The scan reads them once instead.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT t, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(p.tags) AS t
WHERE p.title ||| 'alpha'
GROUP BY t
ORDER BY t;

SELECT t, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(p.tags) AS t
WHERE p.title ||| 'alpha'
GROUP BY t
ORDER BY t;

-- A string and a list at one decode point: the string's term ordinals are fetched first,
-- and the decode reads the string from its ordinals and the list from its doc addresses.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.author, l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY c.author, l
ORDER BY c.author, l;

SELECT c.author, l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY c.author, l
ORDER BY c.author, l;

-- Posts 21 to 30 have no comments, so their rows reach the decode with no doc address for
-- the labels and read as NULL lists.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT l, COUNT(*)
FROM dac_posts p
LEFT JOIN dac_comments c ON c.post_id = p.id
LEFT JOIN LATERAL unnest(c.labels) AS l ON true
WHERE p.title ||| 'alpha'
GROUP BY l
ORDER BY l;

SELECT l, COUNT(*)
FROM dac_posts p
LEFT JOIN dac_comments c ON c.post_id = p.id
LEFT JOIN LATERAL unnest(c.labels) AS l ON true
WHERE p.title ||| 'alpha'
GROUP BY l
ORDER BY l;

-- =============================================================================
-- Other shapes that unnest a list above the join
-- =============================================================================

-- pdb.agg terms on a list unnests it above the join, and the list is read just below.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "c.labels", "order": {"_key": "asc"}}}')
FROM dac_posts p JOIN dac_comments c ON c.post_id = p.id
WHERE p.title ||| 'alpha';

SELECT pdb.agg('{"terms": {"field": "c.labels", "order": {"_key": "asc"}}}')
FROM dac_posts p JOIN dac_comments c ON c.post_id = p.id
WHERE p.title ||| 'alpha';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, l
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
ORDER BY c.id, l
LIMIT 8;

SELECT c.id, l
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
ORDER BY c.id, l
LIMIT 8;

-- pdb.agg terms on a list inside a window over the join.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, pdb.agg('{"terms": {"field": "c.labels", "order": {"_key": "asc"}}}'::jsonb) OVER ()
FROM dac_posts p JOIN dac_comments c ON c.post_id = p.id
WHERE p.title ||| 'alpha'
ORDER BY c.id
LIMIT 3;

SELECT c.id, pdb.agg('{"terms": {"field": "c.labels", "order": {"_key": "asc"}}}'::jsonb) OVER ()
FROM dac_posts p JOIN dac_comments c ON c.post_id = p.id
WHERE p.title ||| 'alpha'
ORDER BY c.id
LIMIT 3;

-- =============================================================================
-- Settings
-- =============================================================================

-- A list has no fetch step, so pinning the fetch into the scan moves the string's fetch
-- and leaves the list where it was.
SET paradedb.defer_column_fetch = off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.author, l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY c.author, l
ORDER BY c.author, l;

SELECT c.author, l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY c.author, l
ORDER BY c.author, l;

RESET paradedb.defer_column_fetch;

-- With the decode never deferred, every list is read in the scan. The rows match the
-- deferred reads above.
SET paradedb.defer_string_decode = off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY l
ORDER BY l;

SELECT l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY l
ORDER BY l;

SELECT t, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(p.tags) AS t
WHERE p.title ||| 'alpha'
GROUP BY t
ORDER BY t;

SELECT c.author, l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY c.author, l
ORDER BY c.author, l;

SELECT l, COUNT(*)
FROM dac_posts p
LEFT JOIN dac_comments c ON c.post_id = p.id
LEFT JOIN LATERAL unnest(c.labels) AS l ON true
WHERE p.title ||| 'alpha'
GROUP BY l
ORDER BY l;

SELECT pdb.agg('{"terms": {"field": "c.labels", "order": {"_key": "asc"}}}')
FROM dac_posts p JOIN dac_comments c ON c.post_id = p.id
WHERE p.title ||| 'alpha';

SELECT c.id, l
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
ORDER BY c.id, l
LIMIT 8;

SELECT c.id, pdb.agg('{"terms": {"field": "c.labels", "order": {"_key": "asc"}}}'::jsonb) OVER ()
FROM dac_posts p JOIN dac_comments c ON c.post_id = p.id
WHERE p.title ||| 'alpha'
ORDER BY c.id
LIMIT 3;

RESET paradedb.defer_string_decode;

-- =============================================================================
-- MPP: the list is read in a worker's fragment
-- =============================================================================

SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.author, l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY c.author, l
ORDER BY c.author, l;

SELECT c.author, l, COUNT(*)
FROM dac_posts p
JOIN dac_comments c ON c.post_id = p.id
CROSS JOIN LATERAL unnest(c.labels) AS l
WHERE p.title ||| 'alpha'
GROUP BY c.author, l
ORDER BY c.author, l;

-- The same list under a join scan: a worker reads it and folds it into the partial terms
-- aggregate, and the leader merges the partials.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, pdb.agg('{"terms": {"field": "c.labels", "order": {"_key": "asc"}}}'::jsonb) OVER ()
FROM dac_posts p JOIN dac_comments c ON c.post_id = p.id
WHERE p.title ||| 'alpha'
ORDER BY c.id
LIMIT 3;

SELECT c.id, pdb.agg('{"terms": {"field": "c.labels", "order": {"_key": "asc"}}}'::jsonb) OVER ()
FROM dac_posts p JOIN dac_comments c ON c.post_id = p.id
WHERE p.title ||| 'alpha'
ORDER BY c.id
LIMIT 3;

RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;

DROP TABLE dac_comments;
DROP TABLE dac_posts;
