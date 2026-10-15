# TopK Aggregate on JOIN, Medium Selectivity (DataFusion)

- **Join**: stackoverflow_posts -> badges
- **Description**: GROUP BY badges.name, a string every badge row carries and that
  repeats across most of them, with COUNT(\*) ordered DESC and LIMIT 10 on a
  join query. This realistically models an Elasticsearch Terms Aggregation on
  a dense key: each matched post fans out to all of its owner's badges, so the
  aggregate sees far more rows than distinct names, and every row carries a
  real string.

## Query Info (statistics from 20m dataset):

- 'regex' selectivity on stackoverflow_posts.body: ~0.90% (~179K matches).
  At 20M scale, per-partition build-side cardinality (~11K-32K rows, <= 20K distinct keys on most
  partitions) causes `HashJoinExec` to emit a dynamic `InListExpr` that exceeds Tantivy's
  `BitsetFromPostings` density gate (`paradedb.term_set_bitset_max_density_multi`) or K-cap
  (`paradedb.hash_join_inlist_pushdown_max_distinct_values`), exercising vectorized `PreFilter`
  `InListExpr` evaluation over fast-field batches.

## Comparison Strategy

Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes this query faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`), but redundant `basescan.sql` variants were pruned from the suite to keep execution fast.
