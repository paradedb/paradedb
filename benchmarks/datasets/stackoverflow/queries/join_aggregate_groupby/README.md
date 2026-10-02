# GROUP BY aggregate on JOIN

- **Join**: stackoverflow_posts → comments
- **Description**: Grouped aggregate (COUNT, SUM) with GROUP BY on a low-cardinality
  dimension (post_type_id). Sorted by sub-aggregation metric (SUM). Exercises the
  DataFusion backend's grouped aggregate pipeline including custom_scan_tlist for scanrelid=0.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%

## Comparison Strategy

Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes this query faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`), but redundant `basescan.sql` variants were pruned from the suite to keep execution fast.
