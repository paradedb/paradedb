# TopK Score DESC with Range Filter, Low Selectivity (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10 with a date range filter.
  This tests Block-Max WAND optimization with a range filter over a large candidate pool.

## Query Info (statistics from 20M dataset; larger or smaller datasets may vary):

- 'code' selectivity on stackoverflow_posts.body: ~75.12% (15.04M matches).
- creation_date >= '2015-01-01' selectivity on stackoverflow_posts.creation_date: ~26.98% (5.40M matches).
- Combined selectivity: ~21.94% (4.39M matches).
