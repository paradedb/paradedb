# Aggregate Sort (Computed Property)

- **Join**: Single Feature (Computed Aggregate)
- **Description**: The feature driving the ORDER BY does not exist until the join is performed. The user is sorting posts by an aggregate of their related comments (e.g., "Most recently active" or "Most interactions"). This requires computing the aggregate (via GROUP BY or LATERAL) before the Top K can be resolved.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%

## Comparison Strategy

Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes this query faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`), but redundant `basescan.sql` variants were pruned from the suite to keep execution fast.
