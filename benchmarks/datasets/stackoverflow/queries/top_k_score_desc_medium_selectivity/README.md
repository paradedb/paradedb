# TopK Score DESC, Medium Selectivity (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10.
  This tests the standard Block-Max WAND optimization with a medium candidate pool.

## Query Info (statistics from 20M dataset; larger or smaller datasets may vary):

- 'use' selectivity on stackoverflow_posts.body: ~27.71% (5.55M matches).
