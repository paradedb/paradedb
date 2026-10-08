# TopK Score DESC with a Partition-Key Range Filter, Partitioned vs Unpartitioned (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10 with `id BETWEEN 20000000 AND 30000000`. `id` is a
  `partition_by` column, so on the partitioned index, segments whose `id` range lies outside the
  filter are skipped using per-segment statistics.

## Query Info

- `partitioned.sql` queries `stackoverflow_posts` (index partitioned by `id,owner_user_id`).
- `unpartitioned.sql` queries `stackoverflow_posts_unpartitioned`: the same rows and the same index
  without `partition_by`, created in `indexes/bm25.sql`.
- The `id` range is chosen so it is expected to hold rows at every sampled dataset size (check the
  row count of the first run).
- `*_single_worker.sql` run the same queries with `max_parallel_workers_per_gather = 0`, so both
  tables use one process and the comparison is not affected by the planner choosing different
  worker counts.
