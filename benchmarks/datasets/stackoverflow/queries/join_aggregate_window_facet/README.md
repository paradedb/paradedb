# Multi-Facet Window Aggregates on JOIN

- **Join**: stackoverflow_posts -> comments
- **Description**: Uses window functions (OVER PARTITION BY) to retrieve Top K hits
  alongside global facet counts across multiple dimensions. This perfectly mimics
  Elasticsearch's faceting behavior, but currently prevents TopK optimization.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%

## Comparison Strategy

Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes this query faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`), but redundant `basescan.sql` variants were pruned from the suite to keep execution fast.
