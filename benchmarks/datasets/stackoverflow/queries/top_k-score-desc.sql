-- Shape: TopK Score (Single Table, BM25)
-- Description: BM25 score ordered DESC and LIMIT 10.
-- This tests the standard Block-Max WAND optimization without tiebreakers.
--
-- Query Info (statistics from 1M dataset; larger datasets may have different values):
-- - 'javascript' selectivity on stackoverflow_posts.body: ~4%
SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10;

-- force single worker
SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10;
