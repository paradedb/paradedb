# TopK Score DESC without a Partition-Key Filter, Partitioned vs Unpartitioned (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10 with no filter. Nothing can be pruned, so this control shows
  the effect of the partitioned segment layout alone; subtract it when reading the filtered cases.

## Query Info

- `partitioned.sql` queries `stackoverflow_posts` (index partitioned by `id,owner_user_id`).
- `unpartitioned.sql` queries `stackoverflow_posts_unpartitioned`: the same rows and the same index
  without `partition_by`, created in `indexes/bm25.sql`.
- `*_single_worker.sql` run the same queries with `max_parallel_workers_per_gather = 0`, so both
  tables use one process and the comparison is not affected by the planner choosing different
  worker counts.
