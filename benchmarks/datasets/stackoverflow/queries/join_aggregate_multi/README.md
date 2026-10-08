# Multiple Aggregates on JOIN

- **Join**: stackoverflow_posts → comments
- **Description**: Multiple aggregate functions (COUNT, SUM, MIN, MAX) on a join.
  Exercises the DataFusion backend's ability to compute multiple aggregates
  in a single pass over the joined data.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%

## Comparison Strategy

Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes this query faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`), but redundant `basescan.sql` variants were pruned from the suite to keep execution fast.
