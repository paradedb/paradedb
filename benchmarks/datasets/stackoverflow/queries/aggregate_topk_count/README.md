# TopK Aggregate (Single Table, Tantivy)

- **Join**: None (single table)
- **Description**: GROUP BY a high-cardinality field with COUNT(\*) ordered DESC
  and LIMIT 10. Tests the Tantivy TopK optimization (TermsAggregation.size=K)
  versus full aggregation + post-hoc sort.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%

## Comparison Strategy

Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes this query faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`). This directory retains `basescan.sql` as a representative single-table Top-K grouped aggregate baseline.
