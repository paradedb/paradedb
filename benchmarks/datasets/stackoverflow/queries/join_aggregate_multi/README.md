# Multiple Aggregates on JOIN

- **Join**: stackoverflow_posts → comments
- **Description**: Multiple aggregate functions (COUNT, SUM, MIN, MAX) on a join.
  Exercises the DataFusion backend's ability to compute multiple aggregates
  in a single pass over the joined data.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%
