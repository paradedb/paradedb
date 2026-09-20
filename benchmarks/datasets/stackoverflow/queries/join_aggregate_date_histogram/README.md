# Date Histogram on JOIN

- **Join**: stackoverflow_posts -> comments
- **Description**: Group by a time bucket (month) using date_trunc. This models the
  extremely common Elasticsearch date_histogram aggregation used for time-series
  analytics. Sorted chronologically.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%
