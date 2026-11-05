-- Regression tests for single-sided non-score fast-field column ordering in RankJoinExec
-- (2-way and 3-way joins across all table positions, ASC/DESC, and NULL handling).

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_rank_join = on;
SET paradedb.rank_join_min_rows = 0;

DROP TABLE IF EXISTS rj_comments CASCADE;
DROP TABLE IF EXISTS rj_posts CASCADE;
DROP TABLE IF EXISTS rj_users CASCADE;

CREATE TABLE rj_users (
    id INT PRIMARY KEY,
    bio TEXT NOT NULL,
    reputation INT
) WITH (autovacuum_enabled = false);

CREATE TABLE rj_posts (
    id INT PRIMARY KEY,
    user_id INT NOT NULL,
    title TEXT NOT NULL,
    views INT
) WITH (autovacuum_enabled = false);

CREATE TABLE rj_comments (
    id INT PRIMARY KEY,
    post_id INT NOT NULL,
    body TEXT NOT NULL,
    upvotes INT
) WITH (autovacuum_enabled = false);

CREATE INDEX rj_users_bm25 ON rj_users USING bm25 (id, bio, reputation);
CREATE INDEX rj_posts_bm25 ON rj_posts USING bm25 (id, user_id, title, views);
CREATE INDEX rj_comments_bm25 ON rj_comments USING bm25 (id, post_id, body, upvotes);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO rj_users
SELECT i,
       CASE WHEN i % 2 = 0 THEN 'rust database search' ELSE 'postgres indexing' END,
       CASE WHEN i % 10 = 0 THEN NULL ELSE (i * 37) % 100 - 20 END
FROM generate_series(1, 25) i;
INSERT INTO rj_users
SELECT i,
       CASE WHEN i % 2 = 0 THEN 'rust database search' ELSE 'postgres indexing' END,
       CASE WHEN i % 10 = 0 THEN NULL ELSE (i * 37) % 100 - 20 END
FROM generate_series(26, 50) i;

INSERT INTO rj_posts
SELECT i,
       ((i * 7) % 50) + 1,
       CASE WHEN i % 3 = 0 THEN 'fast columnar query' ELSE 'bm25 ranking' END,
       CASE WHEN i % 12 = 0 THEN NULL ELSE (i * 53) % 200 - 50 END
FROM generate_series(1, 50) i;
INSERT INTO rj_posts
SELECT i,
       ((i * 7) % 50) + 1,
       CASE WHEN i % 3 = 0 THEN 'fast columnar query' ELSE 'bm25 ranking' END,
       CASE WHEN i % 12 = 0 THEN NULL ELSE (i * 53) % 200 - 50 END
FROM generate_series(51, 100) i;

INSERT INTO rj_comments
SELECT i,
       ((i * 11) % 100) + 1,
       CASE WHEN i % 4 != 0 THEN 'helpful comment' ELSE 'great post' END,
       CASE WHEN i % 15 = 0 THEN NULL ELSE (i * 71) % 300 - 100 END
FROM generate_series(1, 75) i;
INSERT INTO rj_comments
SELECT i,
       ((i * 11) % 100) + 1,
       CASE WHEN i % 4 != 0 THEN 'helpful comment' ELSE 'great post' END,
       CASE WHEN i % 15 = 0 THEN NULL ELSE (i * 71) % 300 - 100 END
FROM generate_series(76, 150) i;

RESET paradedb.global_mutable_segment_rows;
ANALYZE rj_users, rj_posts, rj_comments;

-- ============================================================
-- Test 1: 3-table join ordered by 1st table (u.reputation DESC NULLS LAST)
-- Shape: Inner RankJoinExec(u [SA], p [RA]) -> Outer RankJoinExec(inner [SA], c [RA])
-- ============================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

-- ============================================================
-- Test 2: 3-table join ordered by 2nd table (p.views ASC NULLS LAST)
-- Shape: Inner RankJoinExec(u [RA], p [SA]) -> Outer RankJoinExec(inner [SA], c [RA])
-- ============================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, p.views
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY p.views ASC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, p.views
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY p.views ASC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

-- ============================================================
-- Test 3: 3-table join ordered by 3rd table (c.upvotes DESC NULLS FIRST)
-- Shape: Outer RankJoinExec(Inner RankJoinExec(u [RA], p [RA]) [cascading RA], c [SA])
-- ============================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, c.upvotes
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY c.upvotes DESC NULLS FIRST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, c.upvotes
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY c.upvotes DESC NULLS FIRST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

-- Verify identical results with RankJoin disabled (HashJoinExec baseline)
SET paradedb.enable_rank_join = off;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, p.views
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY p.views ASC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, c.upvotes
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY c.upvotes DESC NULLS FIRST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

-- ============================================================
-- Test 4: With MPP enabled, verify that disabling RankJoin uses MPP (DistributedExec),
-- whereas enabling RankJoin requires a single df-d task (Maximum(1)), preventing MPP
-- distribution so in-memory dynamic filters work.
-- ============================================================
SET paradedb.mpp_min_rows = 0;
SET max_parallel_workers_per_gather = 3;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;

-- Baseline with RankJoin disabled: plans DistributedExec across the 2 segments
SET paradedb.enable_rank_join = off;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

-- With RankJoin enabled: plans single-task RankJoinExec (no DistributedExec)
SET paradedb.enable_rank_join = on;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

-- ============================================================
-- Test 5: 3-table disjunctive (OR) join ordered by 3rd table (c.upvotes DESC NULLS LAST)
-- Shape: ScanMode::Tagged leaves + single-sided RankJoinExec with root JoinFilter
-- ============================================================
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_rank_join = on;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, c.upvotes
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' OR p.title @@@ 'bm25' OR c.body @@@ 'helpful'
ORDER BY c.upvotes DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, c.upvotes
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' OR p.title @@@ 'bm25' OR c.body @@@ 'helpful'
ORDER BY c.upvotes DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

-- Verify identical results with RankJoin disabled (HashJoinExec baseline)
SET paradedb.enable_rank_join = off;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, c.upvotes
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' OR p.title @@@ 'bm25' OR c.body @@@ 'helpful'
ORDER BY c.upvotes DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

-- ============================================================
-- Test 6: RankJoinRule expression shapes and eligibility gates
-- ============================================================
SET paradedb.enable_rank_join = on;
SET paradedb.rank_join_min_rows = 0;

-- 6a: Weighted additive score expression -> plans RankJoinExec
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25'
ORDER BY 2.0 * paradedb.score(u.id) + 3.0 * paradedb.score(p.id) DESC
LIMIT 5;

-- 6b: Multiplicative score expression -> plans RankJoinExec
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25'
ORDER BY paradedb.score(u.id) * paradedb.score(p.id) DESC
LIMIT 5;

-- 6c: Gate 1 rejection (constant-score pdb.all() leg in two-sided score sum) -> HashJoinExec
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.id @@@ pdb.all() AND p.title @@@ 'bm25'
ORDER BY paradedb.score(u.id) + paradedb.score(p.id) DESC
LIMIT 5;

-- 6d: Gate 2 rejection (100x boost imbalance between scored legs) -> HashJoinExec
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust'::pdb.boost(100) AND p.title @@@ 'bm25'
ORDER BY paradedb.score(u.id) + paradedb.score(p.id) DESC
LIMIT 5;

-- 6e: Single-sided score sort over cross-table OR -> rejected (HashJoinExec)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust' OR p.title @@@ 'bm25'
ORDER BY paradedb.score(u.id) DESC
LIMIT 5;

-- 6f: Gate 3 unscored RA selectivity (< 0.08 rejected vs >= 0.08 accepted when rank_join_min_rows > 0)
SET paradedb.rank_join_min_rows = 1;

-- Selective RA side (u.id = 2 matches 1/50 = 0.02 < 0.08) -> rejected (HashJoinExec)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.id = 2 AND p.title @@@ 'bm25'
ORDER BY p.views DESC NULLS LAST
LIMIT 5;

-- Non-selective RA side (u.bio @@@ 'rust' matches 25/50 = 0.50 >= 0.08) -> RankJoinExec
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25'
ORDER BY p.views DESC NULLS LAST
LIMIT 5;

-- ============================================================
-- Test 7: MPP range co-partitioned RankJoinExec
-- - Single-stage co-partitioned 2-way and 3-way joins use RankJoinExec(mode=Partitioned)
--   under DistributedExec.
-- - Multi-key 3-way joins that cannot co-partition into a single stage still use
--   RankJoinExec, falling back to mode=CollectLeft (single partition).
-- ============================================================
DROP INDEX rj_users_bm25, rj_posts_bm25, rj_comments_bm25;

ALTER TABLE rj_comments ADD COLUMN user_id INT;
UPDATE rj_comments c SET user_id = p.user_id FROM rj_posts p WHERE c.post_id = p.id;
ANALYZE rj_users, rj_posts, rj_comments;

SET max_parallel_maintenance_workers = 0;

CREATE INDEX rj_users_bm25 ON rj_users USING bm25 (id, bio, reputation)
WITH (target_segment_count = 3, partition_by = 'id');

CREATE INDEX rj_posts_bm25 ON rj_posts USING bm25 (id, user_id, title, views)
WITH (target_segment_count = 3, partition_by = 'user_id,id');

CREATE INDEX rj_comments_bm25 ON rj_comments USING bm25 (id, post_id, user_id, body, upvotes)
WITH (target_segment_count = 3, partition_by = 'user_id,post_id');

SET paradedb.enable_range_partitioned_join = on;
SET paradedb.enable_rank_join = on;
SET paradedb.rank_join_min_rows = 0;
SET paradedb.mpp_min_rows = 0;
SET max_parallel_workers_per_gather = 3;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;

-- 7a: 2-way range co-partitioned single-sided RankJoinExec -> DistributedExec with mode=Partitioned
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id, p.views
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25'
ORDER BY p.views DESC NULLS LAST, u.id ASC, p.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, p.views
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25'
ORDER BY p.views DESC NULLS LAST, u.id ASC, p.id ASC
LIMIT 5;

-- 7b: 2-way range co-partitioned disjunctive (OR) two-sided score RankJoinExec -> DistributedExec with mode=Partitioned
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust' OR p.title @@@ 'bm25'
ORDER BY paradedb.score(u.id) + paradedb.score(p.id) DESC, u.id ASC, p.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust' OR p.title @@@ 'bm25'
ORDER BY paradedb.score(u.id) + paradedb.score(p.id) DESC, u.id ASC, p.id ASC
LIMIT 5;

-- 7c: 3-way range co-partitioned join on shared key (user_id) -> DistributedExec with mode=Partitioned
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON u.id = c.user_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON u.id = c.user_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

-- 7d: 3-way join on different keys (u.id = p.user_id and p.id = c.post_id) cannot co-partition
-- in a single stage -> still plans RankJoinExec, falling back to mode=CollectLeft (1 partition)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON p.id = c.post_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

-- Verify identical results for 7a, 7b, 7c with RankJoin disabled (HashJoinExec baseline)
SET paradedb.enable_rank_join = off;

SELECT u.id AS u_id, p.id AS p_id, p.views
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25'
ORDER BY p.views DESC NULLS LAST, u.id ASC, p.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
WHERE u.bio @@@ 'rust' OR p.title @@@ 'bm25'
ORDER BY paradedb.score(u.id) + paradedb.score(p.id) DESC, u.id ASC, p.id ASC
LIMIT 5;

SELECT u.id AS u_id, p.id AS p_id, c.id AS c_id, u.reputation
FROM rj_users u
JOIN rj_posts p ON u.id = p.user_id
JOIN rj_comments c ON u.id = c.user_id
WHERE u.bio @@@ 'rust' AND p.title @@@ 'bm25' AND c.body @@@ 'helpful'
ORDER BY u.reputation DESC NULLS LAST, u.id ASC, p.id ASC, c.id ASC
LIMIT 5;

DROP TABLE rj_comments, rj_posts, rj_users;
