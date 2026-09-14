-- =====================================================================
-- Range partitioning in MPP joins and aggregates.
--
-- Exercises:
-- 1. Two-table range co-partitioning where both tables share partition split points.
-- 2. Three-table join where a bridge table (posts) declares multiple partition keys
--    (partition_by = 'user_id,topic_id'). Demonstrates prioritizing the largest tables
--    (users and posts) on user_id in Stage 1 (0 shuffles), and range-repartitioning the
--    intermediate stream into Stage 2 to join against topics's native range partitions
--    because Tier 2 recognizes posts is already committed to another key (1 shuffle total).
-- 3. Three-table aggregation over range-partitioned tables.
-- 4. Asymmetric join where only the larger table is range partitioned (1 shuffle).
-- 5. Asymmetric join where the smaller table is NOT stamped to avoid shuffling the larger table.
--
-- Note on hash_join_single_partition_threshold[_rows] GUCs:
-- In production, DataFusion defaults to broadcasting (CollectLeft) tables below
-- 131,072 rows / 1MB. For physical co-partitioning (Scenario 1), RangeCoPartitionedJoinRule
-- flips CollectLeft to Partitioned automatically because a 0-shuffle local join beats
-- broadcast. For non-co-partitioned or asymmetric joins (Scenarios 2, 4, 5), setting these
-- thresholds to 0 simulates large tables exceeding the broadcast threshold, forcing
-- PartitionMode::Partitioned to exercise range-partitioning adaptation under DataFusion
-- PR #24600 and PR #24766.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_range_partitioned_join TO on;

SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET max_parallel_maintenance_workers TO 0;

-- =====================================================================
-- Setup: fact, bridge, and dimension tables
-- =====================================================================

CREATE TABLE mpp_rp_users (
    id SERIAL PRIMARY KEY,
    user_id INT,
    user_name TEXT
);

CREATE TABLE mpp_rp_posts (
    id SERIAL PRIMARY KEY,
    post_id INT,
    user_id INT,
    topic_id INT,
    title TEXT
);

CREATE TABLE mpp_rp_topics (
    id SERIAL PRIMARY KEY,
    topic_id INT,
    topic_name TEXT
);

CREATE TABLE mpp_rp_categories (
    id SERIAL PRIMARY KEY,
    category_id INT,
    category_name TEXT
);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_rp_users (user_id, user_name)
SELECT g, 'user_' || g
FROM generate_series(1, 100) AS g;

INSERT INTO mpp_rp_posts (post_id, user_id, topic_id, title)
SELECT g,
       ((g * 7) % 100) + 1,
       (g % 10) + 1,
       'post ' || g || ' about tech'
FROM generate_series(1, 300) AS g;

INSERT INTO mpp_rp_topics (topic_id, topic_name)
SELECT g, 'topic_' || g
FROM generate_series(1, 10) AS g;

INSERT INTO mpp_rp_categories (category_id, category_name)
SELECT g, 'category_' || g
FROM generate_series(1, 10) AS g;

RESET paradedb.global_mutable_segment_rows;

ANALYZE mpp_rp_users;
ANALYZE mpp_rp_posts;
ANALYZE mpp_rp_topics;
ANALYZE mpp_rp_categories;

CREATE INDEX mpp_rp_users_idx ON mpp_rp_users
USING paradedb (id, user_id, user_name)
WITH (
    key_field = 'id',
    target_segment_count = 3,
    partition_by = 'user_id',
    numeric_fields = '{"user_id": {"fast": true}}',
    text_fields = '{"user_name": {"fast": true}}'
);

CREATE INDEX mpp_rp_posts_idx ON mpp_rp_posts
USING paradedb (id, post_id, user_id, topic_id, title)
WITH (
    key_field = 'id',
    target_segment_count = 3,
    partition_by = 'user_id,topic_id',
    numeric_fields = '{"user_id": {"fast": true}, "topic_id": {"fast": true}, "post_id": {"fast": true}}',
    text_fields = '{"title": {"fast": true}}'
);

CREATE INDEX mpp_rp_topics_idx ON mpp_rp_topics
USING paradedb (id, topic_id, topic_name)
WITH (
    key_field = 'id',
    target_segment_count = 3,
    partition_by = 'topic_id',
    numeric_fields = '{"topic_id": {"fast": true}}',
    text_fields = '{"topic_name": {"fast": true}}'
);

CREATE INDEX mpp_rp_categories_idx ON mpp_rp_categories
USING paradedb (id, category_id, category_name)
WITH (
    key_field = 'id',
    target_segment_count = 3,
    numeric_fields = '{"category_id": {"fast": true}}',
    text_fields = '{"category_name": {"fast": true}}'
);

-- =====================================================================
-- Scenario 1: Two-table range co-partitioned join (physical co-partitioning)
--
-- Both sides (users: 100 rows, posts: 300 rows) share split points on user_id,
-- achieving mode=Partitioned within a single distributed stage (0 network shuffles).
--
-- Note on threshold GUCs: hash_join_single_partition_threshold_rows / _threshold
-- are NOT set to 0 here. Even though both tables are well below DataFusion's
-- broadcast threshold (131,072 rows / 1MB), RangeCoPartitionedJoinRule recognizes
-- that both sides are physically range co-partitioned with identical split points
-- and flips the join mode from CollectLeft (broadcast) back to Partitioned. A
-- 0-shuffle task-local join is always cheaper than broadcasting across tasks.
-- =====================================================================

-- Baseline
SET max_parallel_workers_per_gather TO 0;

SELECT u.user_name, p.title
FROM mpp_rp_users u
JOIN mpp_rp_posts p ON u.user_id = p.user_id
WHERE p.title @@@ 'post'
ORDER BY u.user_id, p.post_id
LIMIT 5;

-- MPP
SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT u.user_name, p.title
FROM mpp_rp_users u
JOIN mpp_rp_posts p ON u.user_id = p.user_id
WHERE p.title @@@ 'post'
ORDER BY u.user_id, p.post_id
LIMIT 5;

SELECT u.user_name, p.title
FROM mpp_rp_users u
JOIN mpp_rp_posts p ON u.user_id = p.user_id
WHERE p.title @@@ 'post'
ORDER BY u.user_id, p.post_id
LIMIT 5;

-- =====================================================================
-- Scenario 2: Three-table join with multi-field partition_by on bridge
--
-- RangePartitioningRule prioritizes the largest tables (users and posts),
-- co-partitioning them on user_id in Stage 1 (0 shuffles).
--
-- The outer join then joins the intermediate stream (u JOIN p) with topics (10 rows)
-- on topic_id. Because posts is already committed to user_id in Tier 1, the intermediate
-- stream cannot be partition-aligned on topic_id and must be repartitioned anyway.
-- Tier 2 recognizes this and stamps topics with range split points on topic_id even though
-- topics (10 rows) < posts (300 rows).
--
-- In production, if topics has < 131,072 rows, DataFusion's JoinSelection would
-- legitimately choose CollectLeft (broadcast). Setting hash_join_single_partition_threshold_rows
-- and hash_join_single_partition_threshold to 0 simulates a large topics table where
-- broadcast is disqualified, forcing PartitionMode::Partitioned.
--
-- Under PartitionMode::Partitioned, EnforceDistribution (DataFusion PR #24600 / #24766)
-- preserves topics's native range layout in Stage 2 and adapts the intermediate stream from
-- Stage 1 into RepartitionExec: Range(topic_id) via network shuffle, joining against topics
-- in Stage 2 (only 1 network shuffle instead of 2).
-- =====================================================================

-- Baseline
SET max_parallel_workers_per_gather TO 0;

SELECT u.user_name, p.title, t.topic_name
FROM mpp_rp_users u
JOIN mpp_rp_posts p ON u.user_id = p.user_id
JOIN mpp_rp_topics t ON p.topic_id = t.topic_id
WHERE p.title @@@ 'post'
ORDER BY u.user_id, p.post_id
LIMIT 5;

-- MPP
SET max_parallel_workers_per_gather TO 3;
SET paradedb.hash_join_single_partition_threshold_rows = 0;
SET paradedb.hash_join_single_partition_threshold = 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT u.user_name, p.title, t.topic_name
FROM mpp_rp_users u
JOIN mpp_rp_posts p ON u.user_id = p.user_id
JOIN mpp_rp_topics t ON p.topic_id = t.topic_id
WHERE p.title @@@ 'post'
ORDER BY u.user_id, p.post_id
LIMIT 5;

SELECT u.user_name, p.title, t.topic_name
FROM mpp_rp_users u
JOIN mpp_rp_posts p ON u.user_id = p.user_id
JOIN mpp_rp_topics t ON p.topic_id = t.topic_id
WHERE p.title @@@ 'post'
ORDER BY u.user_id, p.post_id
LIMIT 5;

RESET paradedb.hash_join_single_partition_threshold_rows;
RESET paradedb.hash_join_single_partition_threshold;

-- =====================================================================
-- Scenario 3: Three-table aggregate query
--
-- Aggregates over the three-table join (users, posts, topics) grouped by
-- topic_name. The inner join (users JOIN posts) remains co-partitioned on
-- user_id in Stage 2 (0 shuffles). The outer join with topics uses Broadcast
-- (CollectLeft) via Stage 1 because broadcast thresholds are left at default,
-- broadcasting the 10-row topics table across the 2 consumer tasks.
-- =====================================================================

-- Baseline
SET max_parallel_workers_per_gather TO 0;

SELECT t.topic_name, count(*)
FROM mpp_rp_users u
JOIN mpp_rp_posts p ON u.user_id = p.user_id
JOIN mpp_rp_topics t ON p.topic_id = t.topic_id
WHERE p.title @@@ 'post'
GROUP BY t.topic_name
ORDER BY t.topic_name;

-- MPP
SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT t.topic_name, count(*)
FROM mpp_rp_users u
JOIN mpp_rp_posts p ON u.user_id = p.user_id
JOIN mpp_rp_topics t ON p.topic_id = t.topic_id
WHERE p.title @@@ 'post'
GROUP BY t.topic_name
ORDER BY t.topic_name;

SELECT t.topic_name, count(*)
FROM mpp_rp_users u
JOIN mpp_rp_posts p ON u.user_id = p.user_id
JOIN mpp_rp_topics t ON p.topic_id = t.topic_id
WHERE p.title @@@ 'post'
GROUP BY t.topic_name
ORDER BY t.topic_name;

-- =====================================================================
-- Scenario 4: Asymmetric join - larger table range-partitioned (1 shuffle)
--
-- mpp_rp_posts (300 rows) joins mpp_rp_categories (10 rows, unpartitioned) on
-- posts.topic_id = categories.category_id.
--
-- posts declares partition_by = 'user_id,topic_id', while categories has no
-- partition_by. Because posts (300 rows) > categories (10 rows), Tier 2 of
-- RangePartitioningRule stamps posts with range split points on topic_id.
--
-- Because categories is unpartitioned, this join is NOT physically co-partitioned,
-- so RangeCoPartitionedJoinRule cannot flip the mode. In production, a 10-row
-- table would be broadcast (CollectLeft). Setting the threshold GUCs to 0 simulates
-- a large categories table (> 131,072 rows / > 1MB) that cannot be broadcast,
-- forcing PartitionMode::Partitioned.
--
-- Under PartitionMode::Partitioned with DataFusion PR #24600 and PR #24766:
-- 1. EnsureRequirements preserves posts's 2 native range partitions instead of
--    overriding them with Hash when target_partitions (3) > child_partitions (2).
-- 2. categories (Stage 1) is range-repartitioned across the network to match
--    posts's split points ([(6)], 2 partitions).
-- 3. Result: Only 1 network shuffle (categories), while the larger 300-row posts
--    table remains completely local (0 shuffles).
-- =====================================================================

-- Baseline
SET max_parallel_workers_per_gather TO 0;

SELECT p.title, c.category_name
FROM mpp_rp_posts p
JOIN mpp_rp_categories c ON p.topic_id = c.category_id
WHERE p.title @@@ 'post'
ORDER BY p.post_id, c.category_id
LIMIT 5;

-- MPP
SET max_parallel_workers_per_gather TO 3;
SET paradedb.hash_join_single_partition_threshold_rows = 0;
SET paradedb.hash_join_single_partition_threshold = 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.title, c.category_name
FROM mpp_rp_posts p
JOIN mpp_rp_categories c ON p.topic_id = c.category_id
WHERE p.title @@@ 'post'
ORDER BY p.post_id, c.category_id
LIMIT 5;

SELECT p.title, c.category_name
FROM mpp_rp_posts p
JOIN mpp_rp_categories c ON p.topic_id = c.category_id
WHERE p.title @@@ 'post'
ORDER BY p.post_id, c.category_id
LIMIT 5;

RESET paradedb.hash_join_single_partition_threshold_rows;
RESET paradedb.hash_join_single_partition_threshold;

-- =====================================================================
-- Scenario 5: Asymmetric join - smaller table NOT stamped
--
-- mpp_rp_posts (300 rows) joins mpp_rp_topics (10 rows) on
-- posts.post_id = topics.topic_id.
--
-- topics declares partition_by = 'topic_id', but posts has no partition key
-- on post_id. Because topics (10 rows) < posts (300 rows), Tier 2 refuses
-- to stamp topics with range split points because it is the smaller table.
-- Neither table is range-partitioned.
--
-- Setting the threshold GUCs to 0 forces PartitionMode::Partitioned (simulating
-- large tables where broadcast is disqualified). Under Partitioned mode, because
-- neither side has range split points, DataFusion inserts standard Hash repartitions
-- on both sides (2 network shuffles).
--
-- This demonstrates the benefit of Tier 2 refusal: if topics had been stamped,
-- DataFusion PR #24600 would have selected topics as the reference child and
-- range-shuffled the 300-row posts table across the network to match the 10-row
-- topics table. By not stamping topics, we avoid the 1-sided shuffle of the larger
-- table in favor of symmetric hash partitioning.
-- =====================================================================

-- Baseline
SET max_parallel_workers_per_gather TO 0;

SELECT p.title, t.topic_name
FROM mpp_rp_posts p
JOIN mpp_rp_topics t ON p.post_id = t.topic_id
WHERE p.title @@@ 'post'
ORDER BY p.post_id, t.topic_id
LIMIT 5;

-- MPP
SET max_parallel_workers_per_gather TO 3;
SET paradedb.hash_join_single_partition_threshold_rows = 0;
SET paradedb.hash_join_single_partition_threshold = 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.title, t.topic_name
FROM mpp_rp_posts p
JOIN mpp_rp_topics t ON p.post_id = t.topic_id
WHERE p.title @@@ 'post'
ORDER BY p.post_id, t.topic_id
LIMIT 5;

SELECT p.title, t.topic_name
FROM mpp_rp_posts p
JOIN mpp_rp_topics t ON p.post_id = t.topic_id
WHERE p.title @@@ 'post'
ORDER BY p.post_id, t.topic_id
LIMIT 5;

RESET paradedb.hash_join_single_partition_threshold_rows;
RESET paradedb.hash_join_single_partition_threshold;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE mpp_rp_categories;
DROP TABLE mpp_rp_topics;
DROP TABLE mpp_rp_posts;
DROP TABLE mpp_rp_users;
