# TopK Score ASC, Low Selectivity (Single Table, BM25)

- **Description**: BM25 score ordered ASC and LIMIT 10.

## Query Info (statistics from 20M dataset; larger or smaller datasets may vary):

- 'code' selectivity on stackoverflow_posts.body: ~75.12% (15.04M matches).
  Under database convention, this wide filter represents low selectivity (filters out only ~25% of rows).
