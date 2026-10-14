# TopK Score DESC with Range Filter, High Selectivity (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10 with a date range filter.
  This tests Block-Max WAND optimization with a range filter over a narrow candidate pool.

## Query Info (statistics from 20M dataset; larger or smaller datasets may vary):

- 'javascript' selectivity on stackoverflow_posts.body: ~4.05% (811K matches).
- creation_date >= '2015-01-01' selectivity on stackoverflow_posts.creation_date: ~26.98% (5.40M matches).
- Combined selectivity: ~1.02% (205K matches).
