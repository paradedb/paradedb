# TopK Score (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10.
  This tests the standard Block-Max WAND optimization without tiebreakers.

## Query Info (statistics from 1M dataset; larger datasets may have different values):

- 'javascript' selectivity on stackoverflow_posts.body: ~4%
