# TopK Score DESC with Range Filter, Medium Selectivity (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10 with a date range filter.
  This tests Block-Max WAND optimization with a range filter over a medium candidate pool.

## Query Info (statistics from 20M dataset; larger or smaller datasets may vary):

- 'use' selectivity on stackoverflow_posts.body: ~27.71% (5.55M matches).
- creation_date >= '2015-01-01' selectivity on stackoverflow_posts.creation_date: ~26.98% (5.40M matches).
- Combined selectivity: ~7.53% (1.51M matches).
