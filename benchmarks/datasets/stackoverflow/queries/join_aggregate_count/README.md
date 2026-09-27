# Scalar COUNT(\*) on JOIN

- **Join**: stackoverflow_posts → comments
- **Description**: Count total joined rows matching a search predicate.
  This is the simplest aggregate-on-join shape and exercises the
  DataFusion backend's basic scan → join → aggregate pipeline.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%

## Comparison Strategy

Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes this query faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`). This directory retains `basescan.sql` as a representative multi-table join aggregate baseline.
