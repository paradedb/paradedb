# TopK Aggregate on JOIN, Low Selectivity (DataFusion, UUID Join Key)

- Join: `stackoverflow_posts -> badges` over unpartitioned full-entropy `UUID` keys (`badges.user_uuid = stackoverflow_posts.owner_user_uuid`)
- Description: Mirrors `join_aggregate_topk_count_low_selectivity` with deterministic `md5(...)::uuid` join keys (`GROUP BY badges.name`, `ORDER BY COUNT(*) DESC LIMIT 10`).

## Query Info (statistics from 20m dataset):

- `'javascript'` selectivity on `stackoverflow_posts.body`: ~4.05% (~811K matches, fanning out to ~149M joined badge rows).
  Without range co-partitioning on the UUID join keys, build-side cardinality (~811K rows) exercises very large UUID hash joins with high fanout (and cross-process dynamic filter pushdown once supported).

## Variant Omissions

- `range_partitioned.sql` is omitted because the BM25 indexes are not range-partitioned on the UUID columns.
- Native PostgreSQL aggregation and `basescan.sql` are omitted for the same reasons noted in `../join_aggregate_topk_count_low_selectivity/README.md`.
