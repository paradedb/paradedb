-- Shape: Disjunctive Search across Normalized Hierarchy (Score Sort)
-- Join: users → stackoverflow_posts → comments
-- Description: Disjunctive (OR) full-text search across normalized relational tables with relevance score ranking.
--              This pattern represents the normalized relational equivalent of searching across
--              a single flattened/denormalized document (e.g., Elasticsearch index). In ParadeDB,
--              users can execute broad full-text searches across normalized table boundaries
--              without paying the storage, memory, or ingestion cost of denormalizing parent/child entities.
-- Note:        We benchmark the "score sort" variant separately from the "local sort" variant to track
--              the impact of scoring and top-K sorting on disjunctive join scans.
-- TODO:        Implement Block-Max WAND (BMW) scoring optimization for join scans: https://github.com/paradedb/paradedb/issues/5301

-- Query Info:
-- - 'python' selectivity:
--   - comments.text ||| 'python': ~220k matches
--   - stackoverflow_posts.title ||| 'python': ~170k matches
--   - users.about_me ||| 'python': ~24k matches

-- NOTE: It is not possible to execute this query without the joinscan today, because
-- Postgres takes over execution of the entire score-sum expression, which triggers an
-- "unsupported query shape". We leave it here as a duplicate of the query below it, as
-- having our own queries starting from the second position is the convention, and it would
-- be confusing to do otherwise here.
SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  users.id,
  stackoverflow_posts.id,
  comments.id,
  pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'python' OR stackoverflow_posts.title ||| 'python' OR comments.text ||| 'python'
ORDER BY
  pdb_score DESC,
  comments.id DESC
LIMIT 20;

SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  users.id,
  stackoverflow_posts.id,
  comments.id,
  pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'python' OR stackoverflow_posts.title ||| 'python' OR comments.text ||| 'python'
ORDER BY
  pdb_score DESC,
  comments.id DESC
LIMIT 20;

SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; SELECT
  users.id,
  stackoverflow_posts.id,
  comments.id,
  pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score
FROM
  users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id
WHERE
  users.about_me ||| 'python' OR stackoverflow_posts.title ||| 'python' OR comments.text ||| 'python'
ORDER BY
  pdb_score DESC,
  comments.id DESC
LIMIT 20;
