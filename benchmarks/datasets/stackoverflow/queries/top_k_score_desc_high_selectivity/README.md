# TopK Score DESC, High Selectivity (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10.
  This tests the standard Block-Max WAND optimization without tiebreakers.

## Query Info (statistics from 20M dataset; larger or smaller datasets may vary):

- 'javascript' selectivity on stackoverflow_posts.body: ~4.05% (811K matches).
  Under database convention, this narrow filter represents high selectivity (filters out ~96% of rows).
