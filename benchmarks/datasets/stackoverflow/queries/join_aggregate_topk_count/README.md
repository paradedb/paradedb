# TopK Aggregate on JOIN (DataFusion)

- **Join**: stackoverflow_posts -> badges
- **Description**: GROUP BY badges.name, a string every badge row carries and that
  repeats across most of them, with COUNT(\*) ordered DESC and LIMIT 10 on a
  join query. This realistically models an Elasticsearch Terms Aggregation on
  a dense key: each matched post fans out to all of its owner's badges, so the
  aggregate sees far more rows than distinct names, and every row carries a
  real string.

## Query Info (statistics from 20m dataset):

- 'javascript' selectivity on stackoverflow_posts.body: ~4%

## Comparison Strategy

Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes this query faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`), but redundant `basescan.sql` variants were pruned from the suite to keep execution fast.
