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
-- The fixture is sized so that the *absence* of the HashJoin InList
-- dynamic-filter pushdown is visible in the EXPLAIN ANALYZE metrics:
--
--   - users: 500 rows; only ids 1-25 match the users-side text predicate.
--   - posts: 5,000 rows; ALL of them match the posts-side text predicate
--     (the "high selectivity" candidate pool), owner_user_id cycles 1-500,
--     so exactly 250 posts (5,000 * 25/500) are owned by a matching user.
--
-- With the dynamic filter delivered to the probe-side PgSearchScan, the scan
-- emits only the 250 posts owned by matching users and every probe row finds
-- a hash-table match:
--
--   HashJoinExec: ... probe_hit_rate=100% (250/250)
--
-- If dynamic-filter delivery regresses (e.g. filters dropped between plan
-- fragments), the probe scan emits all 5,000 text-matching posts and the same
-- line degrades to probe_hit_rate=5% (250/5.00 K), with input_rows and the
-- probe scan's output_rows ballooning to match.
--
-- Determinism notes:
--   - Serial execution, single-segment indexes, one INSERT ... SELECT per
--     table; the 250 surviving probe rows fit in one scanner batch.
--   - ORDER BY score DESC needs distinct scores in the top 5: five designated
--     surviving posts repeat the search token 6/5/4/3/2 times (BM25 tf grows
--     strictly with repetition), all other posts contain it exactly once and
--     tie strictly below them. The tied mass never reaches the output.

-- Disable parallel workers so plans are deterministic, and force hash joins
-- (the InList dynamic-filter pushdown path composes with HashJoinExec).
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO off;
SET enable_nestloop = off;
SET enable_mergejoin = off;

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
-- users 1-25 (id % 500 + 1 <= 25) and carry strictly descending term
-- frequencies, so they are the deterministic top 5 of the joined result.
INSERT INTO hsel_posts
SELECT
    i,
    'Post ' || i,
    CASE i
        WHEN    5 THEN repeat('beer ', 6)
        WHEN  510 THEN repeat('beer ', 5)
        WHEN 1015 THEN repeat('beer ', 4)
        WHEN 1520 THEN repeat('beer ', 3)
        WHEN 2020 THEN repeat('beer ', 2)
        ELSE 'beer'
    END,
    (i % 500) + 1
FROM generate_series(1, 5000) AS i;

-- Only users 1-25 match ||| 'beer' on about_me.
INSERT INTO hsel_users
SELECT
    i,
    CASE WHEN i <= 25
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
-- ParadeDB Join Scan: probe_hit_rate=100% (250/250) proves the build-side
-- InList dynamic filter reached the probe scan
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

RESET paradedb.enable_join_custom_scan;
RESET enable_mergejoin;
RESET enable_nestloop;
RESET enable_indexscan;
RESET max_parallel_workers_per_gather;
