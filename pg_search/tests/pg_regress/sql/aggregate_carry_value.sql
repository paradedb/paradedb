-- A string column that the group keys decide (they hold its table's primary key)
-- is carried through the aggregate with `pdb_carry_value` instead of being a
-- group key. It stays deferred through the join and the aggregate, and is
-- decoded above the Top K, for the output rows only.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 0;

CREATE TABLE acv_posts (
    id SERIAL PRIMARY KEY,
    body TEXT,
    title TEXT,
    rank INT
);
CREATE TABLE acv_comments (
    id SERIAL PRIMARY KEY,
    post_id INTEGER,
    score INT
);

CREATE INDEX acv_posts_idx ON acv_posts
USING paradedb (id, body, (title::pdb.literal), rank);
CREATE INDEX acv_comments_idx ON acv_comments
USING paradedb (id, post_id, score);

-- Three inserts give three segments. The same title shows up in more than one
-- segment and on more than one post, and some posts have no title.
SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO acv_posts (body, title, rank)
SELECT 'code sample ' || i,
       CASE WHEN i % 5 = 0 THEN NULL ELSE 'title ' || (i % 4) END,
       i % 3
FROM generate_series(1, 10) AS i;
INSERT INTO acv_posts (body, title, rank)
SELECT 'code sample ' || i,
       CASE WHEN i % 5 = 0 THEN NULL ELSE 'title ' || (i % 4) END,
       i % 3
FROM generate_series(11, 20) AS i;
INSERT INTO acv_posts (body, title, rank)
SELECT 'other text ' || i, 'title ' || (i % 4), i % 3
FROM generate_series(21, 25) AS i;

RESET paradedb.global_mutable_segment_rows;

INSERT INTO acv_comments (post_id, score)
SELECT 1 + (i % 25), i
FROM generate_series(1, 100) AS i;

\echo 'Test 1: GROUP BY pk, title: title is carried, and decoded above the Top K'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT p.id, p.title, MAX(c.score) AS best
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.id, p.title
ORDER BY best DESC, p.id
LIMIT 5;

SELECT p.id, p.title, MAX(c.score) AS best
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.id, p.title
ORDER BY best DESC, p.id
LIMIT 5;

\echo 'Test 2: GROUP BY pk only, title in the output'
SELECT p.id, p.title, COUNT(*) AS n
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.id
ORDER BY n DESC, p.id
LIMIT 5;

\echo 'Test 3: every group, NULL titles included'
SELECT p.id, p.title, SUM(c.score) AS total
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.id, p.title
ORDER BY p.id;

\echo 'Test 4: a real MAX(title) compares strings, so title is decoded below the aggregate'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT p.rank, MAX(p.title) AS last_title, COUNT(*)
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.rank
ORDER BY p.rank;

SELECT p.rank, MAX(p.title) AS last_title, COUNT(*)
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.rank
ORDER BY p.rank;

\echo 'Test 5: GROUP BY title without the pk: posts with the same title share a group'
SELECT p.title, COUNT(DISTINCT p.id) AS posts, COUNT(*) AS comments
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.title
ORDER BY p.title NULLS LAST;

\echo 'Test 6: a non-string dependent column stays a group key'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT p.id, p.rank, COUNT(*) AS n
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.id, p.rank
ORDER BY n DESC, p.id
LIMIT 3;

\echo 'Test 7: ORDER BY the carried title'
SELECT p.id, p.title, MAX(c.score) AS best
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.id, p.title
ORDER BY p.title DESC NULLS LAST, p.id
LIMIT 6;

\echo 'Test 8: a deleted post does not reach the groups'
DELETE FROM acv_posts WHERE id = 1;
SELECT p.id, p.title, MAX(c.score) AS best
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.id, p.title
ORDER BY best DESC, p.id
LIMIT 5;

\echo 'Test 9: the MPP path returns the same rows'
SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

SELECT p.id, p.title, SUM(c.score) AS total
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.id, p.title
ORDER BY p.id;

RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;

\echo 'Test 10: the same rows as plain PostgreSQL'
CREATE TEMP TABLE acv_pdb AS
SELECT p.id, p.title, SUM(c.score) AS total, MAX(c.score) AS best
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.id, p.title;

SET paradedb.enable_aggregate_custom_scan TO off;
SET paradedb.enable_join_custom_scan TO off;
CREATE TEMP TABLE acv_pg AS
SELECT p.id, p.title, SUM(c.score) AS total, MAX(c.score) AS best
FROM acv_posts p JOIN acv_comments c ON p.id = c.post_id
WHERE p.body ||| 'code'
GROUP BY p.id, p.title;

SELECT 'only in pdb' AS side, * FROM (SELECT * FROM acv_pdb EXCEPT ALL SELECT * FROM acv_pg) d
UNION ALL
SELECT 'only in pg', * FROM (SELECT * FROM acv_pg EXCEPT ALL SELECT * FROM acv_pdb) d;

RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_join_custom_scan;

DROP TABLE acv_posts, acv_comments;
