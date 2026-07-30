-- Regression coverage for the join_top_k-score-desc-high-selectivity benchmark
-- shape (benchmarks/datasets/stackoverflow/queries/): a score-driven Top K over
-- a two-table join where the text predicate on the probe (scored) side matches
-- a very large candidate pool and the join is what restricts the result:
--
--   SELECT p.id, pdb.score(p.id) AS score, p.title
--   FROM posts p JOIN users u ON p.owner_user_id = u.id
--   WHERE p.body ||| '...' AND u.about_me ||| '...'
--   ORDER BY score DESC LIMIT 5;
--
-- The benchmark's speedup comes from Top K score-threshold pushdown: the TopK
-- sort publishes its current 5th-best score as a dynamic filter that is pushed
-- down to the probe-side PgSearchScan, which then skips lower-scoring
-- candidates at the scanner. This test makes the *absence* of that pushdown
-- visible in the HashJoinExec probe hit rate reported by EXPLAIN ANALYZE.
--
-- Two batching quanta size the fixture:
--   - The scanner consults the pushed-down threshold between batches;
--     paradedb.dynamic_filter_batch_size = 2000 chunks the probe stream.
--   - HashJoinExec coalesces ~8,192 output rows before emitting anything to
--     the TopK sort, so the threshold cannot tighten until that many joined
--     rows exist. The join must produce well over 8,192 rows, or the probe
--     scan finishes before the threshold ever arrives (with a smaller
--     fixture the EXPLAIN output is identical with pushdown disabled).
--
-- Fixture:
--   - users: 500 rows; ids 1-250 match the users-side text predicate.
--   - posts: 40,000 rows; ALL match the posts-side predicate (the "high
--     selectivity" candidate pool); owner_user_id cycles 1-500, so 20,000
--     posts are owned by a matching user. Five designated posts (ids
--     5/10/15/20/25, all survivors) repeat the token 6/5/4/3/2 times, so the
--     top 5 is seeded — with strictly distinct scores — within the first
--     scanner batch; every other post ties strictly below them.
--
-- With pushdown working, the threshold tightens to the 5th designated score
-- as soon as the join emits its first coalesced batch (~9,000 probe rows in),
-- and the scanner skips the rest of the candidate pool:
--
--   HashJoinExec:  ... probe_hit_rate=100% (9.00 K/9.00 K)
--   PgSearchScan:  ... rows_scanned=18.00 K   (of 40,000 candidates)
--
-- If the scanner-level threshold stops being applied, every candidate is
-- surfaced (rows_scanned=40.0 K); if dynamic-filter delivery to the scan
-- breaks entirely, the probe counts balloon as well (probe_hit_rate=50%
-- (20.0 K/40.0 K)). Either way this file's expected output diffs loudly.
--
-- Determinism notes:
--   - Serial execution, single-segment indexes, one INSERT ... SELECT per
--     table; batch boundaries are fixed by the batch-size cap over a fixed
--     doc order.
--   - ORDER BY score DESC needs distinct scores in the top 5: the designated
--     posts' scores strictly descend (BM25 tf grows strictly with token
--     repetition) and the tied mass below them never reaches the output.

-- Disable parallel workers so plans are deterministic, and force hash joins.
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO off;
SET enable_nestloop = off;
SET enable_mergejoin = off;
-- Cap the probe scanner batch size so the TopK score threshold is applied
-- between batches; see the header comment.
SET paradedb.dynamic_filter_batch_size = 2000;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- ===========================================================================
-- Fixture
-- ===========================================================================

DROP TABLE IF EXISTS hsel_posts CASCADE;
DROP TABLE IF EXISTS hsel_users CASCADE;

CREATE TABLE hsel_posts (
    id            INTEGER PRIMARY KEY,
    title         TEXT,
    body          TEXT,
    owner_user_id INTEGER
);

CREATE TABLE hsel_users (
    id       INTEGER PRIMARY KEY,
    about_me TEXT
);

-- Every post matches ||| 'beer'. The five designated posts are all owned by
-- matching users and carry strictly descending term frequencies, so they are
-- the deterministic top 5 of the joined result.
INSERT INTO hsel_posts
SELECT
    i,
    'Post ' || i,
    CASE i
        WHEN  5 THEN repeat('beer ', 6)
        WHEN 10 THEN repeat('beer ', 5)
        WHEN 15 THEN repeat('beer ', 4)
        WHEN 20 THEN repeat('beer ', 3)
        WHEN 25 THEN repeat('beer ', 2)
        ELSE 'beer'
    END,
    (i % 500) + 1
FROM generate_series(1, 40000) AS i;

-- Only users 1-250 match ||| 'beer' on about_me.
INSERT INTO hsel_users
SELECT
    i,
    CASE WHEN i <= 250
        THEN 'brews beer at home'
        ELSE 'enjoys hiking and gardening'
    END
FROM generate_series(1, 500) AS i;

CREATE INDEX hsel_posts_idx ON hsel_posts
USING bm25 (id, title, body, owner_user_id)
WITH (
    key_field = 'id',
    text_fields = '{"title": {"fast": true}, "body": {"fast": true}}',
    numeric_fields = '{"owner_user_id": {"fast": true}}'
);

CREATE INDEX hsel_users_idx ON hsel_users
USING bm25 (id, about_me)
WITH (key_field = 'id');

ANALYZE hsel_posts;
ANALYZE hsel_users;

-- ===========================================================================
-- Baseline: Postgres-driven join (custom scan off), same rows expected
-- ===========================================================================

SET paradedb.enable_join_custom_scan = off;

SELECT
    p.id,
    pdb.score(p.id) AS score,
    p.title
FROM hsel_posts p
JOIN hsel_users u ON p.owner_user_id = u.id
WHERE p.body ||| 'beer' AND u.about_me ||| 'beer'
ORDER BY score DESC
LIMIT 5;

-- ===========================================================================
-- ParadeDB Join Scan: a ~9K-row probe input on the HashJoinExec
-- probe_hit_rate (instead of the full 20K survivors) proves the TopK score
-- threshold reached the probe scan and tightened mid-stream
-- ===========================================================================

SET paradedb.enable_join_custom_scan = on;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT
    p.id,
    pdb.score(p.id) AS score,
    p.title
FROM hsel_posts p
JOIN hsel_users u ON p.owner_user_id = u.id
WHERE p.body ||| 'beer' AND u.about_me ||| 'beer'
ORDER BY score DESC
LIMIT 5;

SELECT
    p.id,
    pdb.score(p.id) AS score,
    p.title
FROM hsel_posts p
JOIN hsel_users u ON p.owner_user_id = u.id
WHERE p.body ||| 'beer' AND u.about_me ||| 'beer'
ORDER BY score DESC
LIMIT 5;

-- ===========================================================================
-- Cleanup
-- ===========================================================================

DROP TABLE IF EXISTS hsel_posts CASCADE;
DROP TABLE IF EXISTS hsel_users CASCADE;

RESET paradedb.dynamic_filter_batch_size;
RESET paradedb.enable_join_custom_scan;
RESET enable_mergejoin;
RESET enable_nestloop;
RESET enable_indexscan;
RESET max_parallel_workers_per_gather;
