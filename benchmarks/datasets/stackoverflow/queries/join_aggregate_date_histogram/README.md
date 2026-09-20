# Date Histogram on JOIN

- **Join**: stackoverflow_posts -> comments
- **Description**: Group by day using date conversion (`p.creation_date::date`).
  Sorted chronologically with LIMIT 30.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%
