# TopK Score DESC, Low Selectivity (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10.
  This tests the standard Block-Max WAND optimization with a very large candidate pool.

## Query Info (statistics from 20M dataset; larger or smaller datasets may vary):

- 'code' selectivity on stackoverflow_posts.body: ~75.12% (15.04M matches).
  Under database convention, this wide filter represents low selectivity (filters out only ~25% of rows).
