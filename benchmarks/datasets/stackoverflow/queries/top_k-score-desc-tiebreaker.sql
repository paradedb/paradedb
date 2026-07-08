-- Shape: TopK Score with Tiebreaker (Single Table, BM25)
-- Description: BM25 score ordered DESC with a primary key tiebreaker and LIMIT 10.
-- This tests that the Block-Max WAND optimization is able to trigger when a score
-- is used as a prefix in the ORDER BY clause along with a tiebreaker.
--
-- Query Info (statistics from 1M dataset; larger datasets may have different values):
-- - 'javascript' selectivity on stackoverflow_posts.body: ~4%
SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC, id LIMIT 10;

-- force single worker
SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC, id LIMIT 10;
