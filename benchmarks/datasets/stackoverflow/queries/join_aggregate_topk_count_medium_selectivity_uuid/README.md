# TopK Aggregate on JOIN, Medium Selectivity (DataFusion, UUID Join Key)

- Join: `stackoverflow_posts -> badges` over unpartitioned full-entropy `UUID` keys (`badges.user_uuid = stackoverflow_posts.owner_user_uuid`)
- Description: Mirrors `join_aggregate_topk_count_medium_selectivity` with deterministic `md5(...)::uuid` join keys (`GROUP BY badges.name`, `ORDER BY COUNT(*) DESC LIMIT 10`).

## Query Info (statistics from 20m dataset):

- `'regex'` selectivity on `stackoverflow_posts.body`: ~0.90% (~179K matches).
  Without range co-partitioning on the UUID join keys, build-side cardinality (~179K rows) exercises medium/large UUID hash joins (and cross-process dynamic filter pushdown once supported).

## Variant Omissions

- `range_partitioned.sql` is omitted because the BM25 indexes are not range-partitioned on the UUID columns.
- Native PostgreSQL aggregation and `basescan.sql` are omitted for the same reasons noted in `../join_aggregate_topk_count_medium_selectivity/README.md`.
