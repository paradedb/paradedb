-- Shape: Disjunctive Search Scalar COUNT(*) Aggregate on JOIN
-- Join: users → stackoverflow_posts → comments
-- Description: Scalar COUNT(*) aggregate over a disjunctive (OR) full-text search across normalized relational tables.
--              This pattern represents the normalized relational equivalent of searching across
--              a single flattened/denormalized document (e.g., Elasticsearch index). In ParadeDB,
--              users can execute broad full-text searches across normalized table boundaries
--              without paying the storage, memory, or ingestion cost of denormalizing parent/child entities.

-- Query Info (statistics from 20m dataset):
-- - 'python' selectivity:
--   - comments.text ||| 'python': ~220k matches
--   - stackoverflow_posts.title ||| 'python': ~170k matches
--   - users.about_me ||| 'python': ~24k matches

-- Postgres default plan (custom scan off)
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*)
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE u.about_me ||| 'python' OR p.title ||| 'python' OR c.text ||| 'python';

-- DataFusion aggregate scan
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*)
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE u.about_me ||| 'python' OR p.title ||| 'python' OR c.text ||| 'python';

-- DataFusion aggregate scan with range partitioned join
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; SELECT COUNT(*)
FROM users u
JOIN stackoverflow_posts p ON u.id = p.owner_user_id
JOIN comments c ON p.id = c.post_id
WHERE u.about_me ||| 'python' OR p.title ||| 'python' OR c.text ||| 'python';
